$ErrorActionPreference = 'Stop'

$baseDir = $env:CLICK_AUTOMATION_BASE_DIR
$clipboardPath = Join-Path $baseDir 'clipboard_output.txt'
$screenshotsPath = Join-Path $baseDir 'screenshots'

if (Test-Path -LiteralPath $clipboardPath) {
    $lines = @(
        [System.IO.File]::ReadAllLines($clipboardPath) |
            ForEach-Object { $_.Trim() } |
            Where-Object { $_.Length -gt 0 }
    )
    $content = [System.IO.File]::ReadAllText($clipboardPath)
} else {
    $lines = @()
    $content = ''
}

$pngNames = @()
if (Test-Path -LiteralPath $screenshotsPath) {
    $pngNames = @(
        Get-ChildItem -LiteralPath $screenshotsPath -File -Filter '*.png' |
            ForEach-Object { $_.BaseName }
    )
}

$matched = @(
    $lines | Where-Object {
        $line = $_
        $pngNames | Where-Object { $_ -like "*$line*" } | Select-Object -First 1
    }
)
$missing = @(
    $lines | Where-Object {
        $line = $_
        -not ($pngNames | Where-Object { $_ -like "*$line*" } | Select-Object -First 1)
    }
)

$payload = [ordered]@{
    summary = "匹配数量: $($matched.Count) / TXT 行数: $($lines.Count) / PNG 数量: $($pngNames.Count)"
    missing_lines = @($missing)
    content = $content
}
$payload | ConvertTo-Json -Compress
