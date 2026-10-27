# Patch Package Protocol

The main executable discovers patch packages from the `patches` directory beside
the executable. Each immediate child directory is one package and must contain
`manifest.json`.

Example manifest:

```json
{
  "id": "example",
  "version": "1.0.0",
  "name": "Example patch",
  "actions": [
    {
      "id": "example-action",
      "name": "Example action",
      "command": "run.ps1"
    }
  ],
  "tabs": [
    {
      "id": "example-tab",
      "title": "Example tab",
      "command": "tab.ps1",
      "refresh_seconds": 5
    }
  ]
}
```

`command` may be a relative executable path, a `.ps1` script, a `.sh` script,
or an object with `command` and `args`. The host rejects absolute paths,
missing files, and paths that leave the package directory.

Patch commands receive:

- `CLICK_AUTOMATION_BASE_DIR`: directory containing the main executable.
- `CLICK_AUTOMATION_PATCH_DIR`: current patch package directory.
- `CLICK_AUTOMATION_PATCH_ID`: package ID.
- `CLICK_AUTOMATION_OPERATION_ID`: action or Tab ID.

Action commands may write normal log text to stdout. A successful Tab command
must write one JSON object with optional `summary`, `missing_lines`, and
`content` fields. The host renders these fields in a generic Tab view and
refreshes them independently from the built-in GUI.
