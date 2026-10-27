# Clipboard tools patch

This package is discovered from the `patches` directory beside the executable.

- `normalize-clipboard.ps1` is exposed as an action step.
- `clipboard-report.ps1` is exposed as a tab and returns a JSON snapshot.

Patch commands receive the executable directory through
`CLICK_AUTOMATION_BASE_DIR`. Replace this folder with a compatible package to
deploy an incremental feature without rebuilding the main GUI executable.
