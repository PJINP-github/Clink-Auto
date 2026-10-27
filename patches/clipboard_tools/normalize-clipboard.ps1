$ErrorActionPreference = 'Stop'

$baseDir = $env:CLICK_AUTOMATION_BASE_DIR
$outputPath = Join-Path $baseDir 'clipboard_output.txt'

if (Test-Path -LiteralPath $outputPath) {
    $lines = [System.IO.File]::ReadAllLines($outputPath)
} else {
    $lines = @()
}

$seen = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::Ordinal)
$result = [System.Collections.Generic.List[string]]::new()
foreach ($line in $lines) {
    $normalized = $line.Trim()
    if ($normalized.Length -gt 0 -and $seen.Add($normalized)) {
        $result.Add($normalized)
    }
}

if (Test-Path -LiteralPath $outputPath) {
    Remove-Item -LiteralPath $outputPath -Force
}

$utf8NoBom = [System.Text.UTF8Encoding]::new($false)
[System.IO.File]::WriteAllLines($outputPath, $result, $utf8NoBom)
Write-Output ("normalized_lines={0}" -f $result.Count)
