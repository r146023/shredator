# Exit Codes

Shredator uses explicit process exit codes.

| Exit code | Status | Meaning |
|---:|---|---|
| `0` | `completed` | The requested operation completed. |
| `1` | `failed` | Runtime or file operation failed. |
| `2` | `usage` | Invalid CLI usage. |
| `3` | `cancelled` | User cancelled or machine mode refused to prompt without `--force`. |

## Exit code 0: completed

Examples:

```bash
shredator ./secret.txt --force
shredator ./secret.txt --force --json
```

A zero exit code means Shredator completed its operation. It does not guarantee forensic unrecoverability.

For strict automation, also check the machine-readable summary:

```text
success == true
status == "completed"
summary.errors == 0
summary.warnings == 0
```

## Exit code 1: failed

Common causes:

- Path exists but cannot be opened for writing.
- Permission denied.
- File locked by another process.
- Directory cannot be removed after retries.
- File-list run had one or more failed processed paths.
- Unexpected I/O error.

Example failure object:

```json
{
  "schema": "shredator.machine.v1",
  "type": "summary",
  "success": false,
  "status": "failed",
  "exit_code": 1,
  "summary": {
    "paths_failed": 1,
    "errors": 1
  }
}
```

## Exit code 2: usage

Common causes:

- No path and no `--file-list`.
- Unknown flag.
- Missing flag value.
- Invalid pass count.
- Invalid output format.
- Multiple positional paths.

Examples:

```bash
shredator
shredator ./a ./b
shredator ./secret.txt --passes banana
shredator ./secret.txt --output xml
```

## Exit code 3: cancelled

Text mode:

- User answered anything other than `y` to the confirmation prompt.

Machine mode:

- Confirmation would have been required and `--force` was not supplied.

Example:

```bash
shredator ./important.pdf --json
```

Possible outcome:

```json
{
  "success": false,
  "status": "cancelled",
  "exit_code": 3,
  "summary": {
    "paths_skipped": 1,
    "warnings": 1
  }
}
```

## Shell handling

```bash
shredator ./secret.txt --force --json > result.json
code=$?

case "$code" in
  0) echo "completed" ;;
  1) echo "failed" ;;
  2) echo "usage error" ;;
  3) echo "cancelled" ;;
  *) echo "unexpected exit code: $code" ;;
esac
```

## PowerShell handling

```powershell
& shredator.exe .\secret.txt --force --json | Out-File result.json -Encoding utf8
$code = $LASTEXITCODE

switch ($code) {
    0 { "completed" }
    1 { "failed" }
    2 { "usage error" }
    3 { "cancelled" }
    default { "unexpected exit code: $code" }
}
```
