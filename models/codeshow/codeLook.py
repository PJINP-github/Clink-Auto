#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""codeLook: 读取项目代码/配置路径与文件内容，按大小拆分为 seg-N.txt。

最小化输出：去掉空行和行尾空白。
三类目录：exclude_dirs / skip_content_dirs / 默认读取。
额外输出内容读取树：segments/tree.txt（+ 标记读取内容的文件）。

本文件放在 models/codeshow/ 下，靠 find_project_root 自动向上
定位项目根（Cargo.toml + src/），无需手写 --project-root。

用法：
    python codeLook.py                          # 按同目录 look.yaml 执行
    python codeLook.py --dry-run                # 只统计，不写文件
    python codeLook.py --stdout                 # 不分片，直接打印
    python codeLook.py --project-root D:\\X     # 指定项目根
    python codeLook.py --config look.yaml       # 指定配置
    python codeLook.py --split-size 50000       # 覆盖分片大小
    python codeLook.py --segments-dir segments  # 覆盖输出目录
    python codeLook.py --no-compact             # 关闭压缩
    python codeLook.py --no-tree                # 不生成 tree.txt
    python codeLook.py --tree-limit 300         # 预览行数（默认 120）
"""
from __future__ import annotations

import argparse
import fnmatch
import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
TREE_PREVIEW_DEFAULT = 120

LANG_MAP = {
    'py': 'python', 'rs': 'rust', 'js': 'javascript', 'ts': 'typescript',
    'html': 'html', 'css': 'css', 'json': 'json', 'toml': 'toml',
    'yaml': 'yaml', 'yml': 'yaml', 'md': 'markdown',
    'sh': 'bash', 'bat': 'batch', 'cmd': 'batch', 'sql': 'sql',
    'ps1': 'powershell',
}


# ---------------- 项目根 / 配置 ----------------

def find_project_root(start: Path) -> Path | None:
    """从 start 向上查找项目根。"""
    current = start.resolve()
    if current.is_file():
        current = current.parent
    while True:
        # Rust 项目根：Cargo.toml + src/
        if (current / 'Cargo.toml').is_file() and (current / 'src').is_dir():
            return current
        # 兼容其他项目：src/handlers/workspace.rs
        if (current / 'src' / 'handlers' / 'workspace.rs').is_file():
            return current
        # 兜底：只要 Cargo.toml
        if (current / 'Cargo.toml').is_file():
            return current
        if current.parent == current:
            return None
        current = current.parent


def load_config(path: Path) -> dict:
    try:
        import yaml
    except ImportError as exc:
        print(f"[ERROR] 需要 PyYAML: {exc}", file=sys.stderr)
        sys.exit(1)
    with path.open('r', encoding='utf-8-sig') as f:
        return yaml.safe_load(f) or {}


# ---------------- 规则归一化 ----------------

def normalize_prefix(value) -> str:
    return str(value or '').strip().strip('/').replace('\\', '/')


def normalize_extensions(raw) -> set[str]:
    out: set[str] = set()
    for e in (raw or []):
        e = str(e).strip()
        if not e:
            continue
        if not e.startswith('.'):
            e = '.' + e
        out.add(e.lower())
    return out


# ---------------- 排除 / 规则匹配 ----------------

def should_exclude(rel: str, patterns: list[str]) -> bool:
    """判断 rel 是否命中任一 exclude pattern。

    匹配三类目标：
      1. 完整路径
      2. 任意前缀
      3. 任意单段（写 `target` 即可排除任意深度的 target 目录）
    支持 * 通配。
    """
    if not patterns:
        return False
    parts = rel.split('/')
    targets = [rel]
    for i in range(1, len(parts) + 1):
        targets.append('/'.join(parts[:i]))
    targets.extend(parts)
    for pattern in patterns:
        for t in targets:
            if fnmatch.fnmatchcase(t, pattern):
                return True
    return False


def prefix_match(prefix: str, rel: str) -> bool:
    if not prefix:
        return True
    if rel == prefix:
        return True
    return rel.startswith(prefix + '/')


def build_rule_context(config: dict):
    default_exts = normalize_extensions(config.get('default_extensions') or [])
    skip_dirs: list[str] = []
    for raw in (config.get('skip_content_dirs') or []):
        p = normalize_prefix(raw)
        if p:
            skip_dirs.append(p)
    content_rules: list[tuple[str, set[str]]] = []
    for rule in (config.get('content_rules') or []):
        if not isinstance(rule, dict):
            continue
        prefix = normalize_prefix(rule.get('prefix') or '')
        exts = normalize_extensions(rule.get('extensions') or [])
        content_rules.append((prefix, exts))
    return default_exts, skip_dirs, content_rules


def extensions_for(rel: str, default_exts: set[str],
                   skip_dirs: list[str],
                   content_rules: list[tuple[str, set[str]]]) -> set[str]:
    for skip in skip_dirs:
        if prefix_match(skip, rel):
            return set()
    result = default_exts
    for prefix, exts in content_rules:
        if prefix_match(prefix, rel):
            result = exts
    return result


# ---------------- 收集目录树 ----------------

def collect_entries(root: Path, exclude_patterns: list[str]) -> list[tuple[str, int, bool]]:
    entries: list[tuple[str, int, bool]] = []

    def walk(path: Path, depth: int) -> None:
        try:
            children = list(path.iterdir())
        except (PermissionError, OSError):
            return
        children.sort(key=lambda p: (not p.is_dir(), p.name.lower()))
        for child in children:
            rel = child.relative_to(root).as_posix()
            if should_exclude(rel, exclude_patterns):
                continue
            is_dir = child.is_dir()
            entries.append((rel, depth, is_dir))
            if is_dir:
                walk(child, depth + 1)

    walk(root, 0)
    return entries


def collect_content_files(entries, default_exts, skip_dirs, content_rules) -> set[str]:
    files: set[str] = set()
    for rel, _, is_dir in entries:
        if is_dir:
            continue
        exts = extensions_for(rel, default_exts, skip_dirs, content_rules)
        if not exts:
            continue
        if Path(rel).suffix.lower() in exts:
            files.add(rel)
    return files


# ---------------- 文本压缩 ----------------

def compact_text(text: str) -> str:
    out = []
    for line in text.split('\n'):
        stripped = line.rstrip()
        if stripped:
            out.append(stripped)
    return '\n'.join(out)


def compact_all(text: str) -> str:
    out = [line.rstrip() for line in text.split('\n') if line.strip()]
    return '\n'.join(out) + '\n'


def read_file_text(path: Path, compact: bool) -> str:
    try:
        raw = path.read_text(encoding='utf-8', errors='replace')
        if compact:
            return compact_text(raw)
        return raw.rstrip('\n')
    except Exception as exc:
        return f"<读取失败: {type(exc).__name__}: {exc}>"


# ---------------- 组装输出 ----------------

def build_output(root: Path, entries, content_files: set[str], compact: bool) -> str:
    lines: list[str] = [f"{root.name}/"]
    for rel, depth, is_dir in entries:
        indent = '--' * (depth + 1)
        name = rel.rsplit('/', 1)[-1]
        if is_dir:
            lines.append(f"{indent}{name}/")
            continue
        lines.append(f"{indent}{name}")
        if rel in content_files:
            ext = Path(rel).suffix.lower().lstrip('.')
            lang = LANG_MAP.get(ext, '')
            fence = '````'
            lines.append(f"{fence}{lang}" if lang else fence)
            lines.append(read_file_text(root / rel, compact))
            lines.append(fence)
    text = '\n'.join(lines) + '\n'
    return compact_all(text) if compact else text


# ---------------- 内容读取树 ----------------

def build_content_tree(content_files: set[str], root_name: str) -> str:
    """只显示读取内容的文件的 tree。+ 标记文件；目录只在含 + 文件时出现。"""
    if not content_files:
        return f"{root_name}/\n(没有文件被读取内容)\n"

    tree: dict = {}
    for rel in sorted(content_files):
        parts = rel.split('/')
        node = tree
        for p in parts[:-1]:
            node = node.setdefault(p, {})
        node.setdefault('__files__', []).append(parts[-1])

    lines: list[str] = [f"{root_name}/"]

    def render(node: dict, depth: int) -> None:
        dirs = sorted(k for k in node.keys() if k != '__files__')
        files = sorted(node.get('__files__', []))
        for d in dirs:
            indent = '--' * (depth + 1)
            lines.append(f"{indent}{d}/")
            render(node[d], depth + 1)
        for f in files:
            indent = '--' * (depth + 1)
            lines.append(f"{indent}+{f}")

    render(tree, 0)
    return '\n'.join(lines) + '\n'


# ---------------- 分片 ----------------

def split_text(text: str, limit: int) -> list[str]:
    if limit <= 0:
        return [text]
    lines = text.split('\n')
    chunks: list[str] = []
    current: list[str] = []
    current_len = 0
    for line in lines:
        line_len = len(line) + 1
        if current and current_len + line_len > limit:
            chunks.append('\n'.join(current) + '\n')
            current = [line]
            current_len = line_len
        else:
            current.append(line)
            current_len += line_len
    if current:
        chunks.append('\n'.join(current) + '\n')
    return chunks or ['\n']


def clear_old_segments(dir: Path) -> int:
    if not dir.is_dir():
        return 0
    removed = 0
    for path in dir.glob('seg-*.txt'):
        try:
            path.unlink()
            removed += 1
        except OSError:
            pass
    return removed


def write_segments(dir: Path, chunks: list[str]) -> list[Path]:
    dir.mkdir(parents=True, exist_ok=True)
    written: list[Path] = []
    for i, chunk in enumerate(chunks, start=1):
        path = dir / f"seg-{i}.txt"
        path.write_text(chunk, encoding='utf-8', newline='\n')
        written.append(path)
    return written


# ---------------- 主流程 ----------------

def main():
    parser = argparse.ArgumentParser(description='codeLook: 项目快照，按大小拆分为 seg-N.txt')
    parser.add_argument('--project-root', default='')
    parser.add_argument('--config', default=str(SCRIPT_DIR / 'look.yaml'))
    parser.add_argument('--stdout', action='store_true', help='不分片，直接打印')
    parser.add_argument('--split-size', type=int, default=0)
    parser.add_argument('--segments-dir', default='')
    parser.add_argument('--dry-run', action='store_true', help='只统计，不写文件')
    parser.add_argument('--no-compact', action='store_true',
                        help='关闭压缩（保留原空行和行尾空白）')
    parser.add_argument('--no-tree', action='store_true',
                        help='不生成 tree.txt')
    parser.add_argument('--tree-limit', type=int, default=TREE_PREVIEW_DEFAULT,
                        help=f'stdout 中 tree 预览行数（默认 {TREE_PREVIEW_DEFAULT}，0=全打印）')
    args = parser.parse_args()

    # 项目根
    if args.project_root:
        project_root = Path(args.project_root).resolve()
    else:
        found = find_project_root(SCRIPT_DIR)
        if found is None:
            print(f"[ERROR] 从 {SCRIPT_DIR} 向上未找到项目根", file=sys.stderr)
            print("        可用 --project-root 手动指定", file=sys.stderr)
            return 1
        project_root = found

    # 配置
    config_path = Path(args.config).resolve()
    if not config_path.is_file():
        print(f"[ERROR] 配置文件不存在: {config_path}", file=sys.stderr)
        return 1
    config = load_config(config_path)

    if args.no_compact:
        compact = False
    else:
        compact = bool(config.get('compact', True))

    exclude_patterns = [str(p) for p in (config.get('exclude_dirs') or [])]
    default_exts, skip_dirs, content_rules = build_rule_context(config)

    # 收集 + 组装
    entries = collect_entries(project_root, exclude_patterns)
    content_files = collect_content_files(entries, default_exts, skip_dirs, content_rules)
    output_text = build_output(project_root, entries, content_files, compact)

    if args.stdout:
        sys.stdout.write(output_text)
        return 0

    if args.dry_run:
        print(f"[DRY] 项目根: {project_root}")
        print(f"[DRY] 条目: {len(entries)} 个路径, {len(content_files)} 个文件打印内容")
        print(f"[DRY] 压缩: {'开' if compact else '关'}")
        print(f"[DRY] 全局扩展名: {sorted(default_exts)}")
        print(f"[DRY] 只读路径目录: {len(skip_dirs)} 个")
        print(f"[DRY] 精细规则: {len(content_rules)} 条")
        print(f"[DRY] 输出字符数: {len(output_text)}")
        return 0

    # 分片
    raw_split = args.split_size if args.split_size > 0 else config.get('split_size', 100000)
    try:
        split_size = int(raw_split)
    except (TypeError, ValueError):
        split_size = 100000
    if split_size <= 0:
        split_size = 100000

    segments_dir_value = args.segments_dir or str(config.get('segments_dir') or 'segments')
    segments_dir = (SCRIPT_DIR / segments_dir_value).resolve()

    removed = clear_old_segments(segments_dir)
    chunks = split_text(output_text, split_size)
    written = write_segments(segments_dir, chunks)
    total = sum(p.stat().st_size for p in written)

    # 内容读取树
    tree_text = build_content_tree(content_files, project_root.name)
    tree_path = segments_dir / 'tree.txt'
    if not args.no_tree:
        tree_path.write_text(tree_text, encoding='utf-8', newline='\n')
    tree_lines = tree_text.split('\n')

    # 统计输出
    total_kb = total / 1024
    print(f"[OK] 项目根: {project_root}")
    print(f"     条目: {len(entries)} 个路径, {len(content_files)} 个文件打印内容")
    print(f"     压缩: {'开' if compact else '关'}")
    print(f"     分片: {len(written)} 个 -> {segments_dir}")
    print(f"     上限: {split_size} 字符/片")
    print(f"     清理旧分片: {removed} 个")
    print(f"     大小: {total} 字节 ({total_kb:.1f} KB, {total_kb/1024:.2f} MB)")
    if not args.no_tree:
        print(f"     读取树: {tree_path}  ({len(tree_lines)} 行)")
    print()

    # tree 预览
    if not args.no_tree:
        print("--- 内容读取树（+ = 读取内容的文件；未列出的文件只输出路径）---")
        limit = args.tree_limit
        if limit <= 0 or len(tree_lines) <= limit:
            sys.stdout.write(tree_text)
        else:
            for line in tree_lines[:limit]:
                print(line)
            remain = len(tree_lines) - limit
            print(f"... (共 {len(tree_lines)} 行，剩余 {remain} 行见 {tree_path})")

    return 0


if __name__ == '__main__':
    sys.exit(main())