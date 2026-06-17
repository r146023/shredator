# Machine-Readable Output

Shredator supports structured output for wrappers, monitoring systems, and GUI integrations.

## Modes

| Mode | Flag | Shape |
|---|---|---|
| JSON | `--json`, `--machine-readable`, `--output json` | One final JSON object containing summary and all retained events. |
| JSONL | `--jsonl`, `--ndjson`, `--output jsonl` | One JSON object per event, then one final summary object. |

## Recommended wrapper defaults

For short operations:

```bash
shredator ./secret.txt --force --json
```

For long operations where progress matters:

```bash
shredator ./large-directory --force --jsonl
```

## Non-interactive contract

Machine-readable modes do not prompt. This is deliberate. A JSON parser should never have to deal with a `Continue? (y/n)` prompt mixed into stdout.

If confirmation would be required and `--force` is absent, Shredator emits a structured `confirmation_required` warning and exits with status `cancelled`.

## JSON mode

JSON mode emits a single final object:

```json
{
  "schema": "shredator.machine.v1",
  "type": "summary",
  "success": true,
  "status": "completed",
  "exit_code": 0,
  "duration_ms": 18,
  "output_format": "json",
  "summary": {
    "files_shredded": 1,
    "directories_removed": 0,
    "paths_successful": 1,
    "paths_failed": 0,
    "paths_skipped": 0,
    "bytes_seen": 1024,
    "bytes_overwritten": 3072,
    "overwrite_passes_completed": 3,
    "warnings": 0,
    "errors": 0
  },
  "events": []
}
```

The `events` array contains structured events emitted during the run.

## JSONL mode

JSONL mode emits events as they happen:

```jsonl
{"type":"event","level":"info","event":"file_start_requested","message":"Shredding file: ./secret.txt (using 3 passes)","path":"./secret.txt","passes":3,"pattern":"random"}
{"type":"event","level":"info","event":"file_start","message":"Shredding ./secret.txt (1024 bytes) with 3 passes","path":"./secret.txt","size_bytes":1024,"passes":3,"pattern":"random"}
{"schema":"shredator.machine.v1","type":"summary","success":true,"status":"completed","exit_code":0,"duration_ms":18,"output_format":"jsonl","summary":{"files_shredded":1,"directories_removed":0,"paths_successful":1,"paths_failed":0,"paths_skipped":0,"bytes_seen":1024,"bytes_overwritten":3072,"overwrite_passes_completed":3,"warnings":0,"errors":0}}
```

Each line is a standalone JSON object. The final line is the summary object.

## Top-level summary object

| Field | Type | Meaning |
|---|---|---|
| `schema` | string | Schema identifier. Current value: `shredator.machine.v1`. |
| `type` | string | `summary` for the final object. |
| `success` | bool | True only when status is `completed`. |
| `status` | string | `completed`, `failed`, `cancelled`, or `usage`. |
| `exit_code` | integer | Process exit code Shredator uses. |
| `duration_ms` | integer | Total elapsed process duration in milliseconds. |
| `output_format` | string | `json` or `jsonl`. |
| `summary` | object | Counters for the run. |
| `events` | array | Present in JSON mode. Not present in JSONL summary. |

## Summary fields

| Field | Type | Meaning |
|---|---|---|
| `files_shredded` | integer | Successfully removed files. |
| `directories_removed` | integer | Successfully removed directories. |
| `paths_successful` | integer | Successful top-level paths. |
| `paths_failed` | integer | Failed top-level paths. |
| `paths_skipped` | integer | Skipped paths/lines/items. |
| `bytes_seen` | integer | Total original file bytes seen. |
| `bytes_overwritten` | integer | Total bytes written during completed overwrite passes. |
| `overwrite_passes_completed` | integer | Number of completed overwrite passes. |
| `warnings` | integer | Warning event count. |
| `errors` | integer | Error event count. |

## Event object

Every event has at least:

| Field | Type | Meaning |
|---|---|---|
| `type` | string | Always `event` for event objects. |
| `level` | string | `info`, `warning`, or `error`. |
| `event` | string | Stable-ish event name. |
| `message` | string | Human-readable description. |

Events may include additional fields such as:

- `path`
- `original_path`
- `renamed_path`
- `passes`
- `pass`
- `total_passes`
- `pattern`
- `size_bytes`
- `bytes_written`
- `elapsed_ms`
- `error`
- `line`
- `force_required`
- `renamed_before_delete`

Wrappers should ignore unknown fields.

## Event names

Current event names include:

| Event | Meaning |
|---|---|
| `help_requested` | Help was requested in machine mode. |
| `usage_error` | Invalid CLI usage. |
| `fatal_error` | Unhandled fatal I/O or runtime error. |
| `path_not_found` | Single requested path does not exist. |
| `confirmation_required` | Machine mode refused to prompt without `--force`. |
| `operation_cancelled` | User cancelled a text-mode confirmation prompt. |
| `operation_failed` | Single requested path failed. |
| `directory_start` | Directory operation started. |
| `directory_skipped_depth` | Directory skipped due to `--max-depth`. |
| `path_skipped_excluded` | Path skipped by `--exclude`. |
| `path_skipped_not_included` | File skipped because it did not match `--include`. |
| `directory_remove_start` | Directory removal started. |
| `directory_remove_retry` | Directory removal retry occurred. Verbose-only. |
| `directory_removed` | Directory removed. |
| `file_start_requested` | Single-file operation requested. |
| `file_start` | File shredding started. |
| `empty_file_remove` | Empty file removed directly. |
| `overwrite_pass_start` | Overwrite pass started. |
| `overwrite_pass_complete` | Overwrite pass completed. |
| `overwrite_pass_failed` | Overwrite pass failed. |
| `truncate_start` | Truncation started. |
| `truncate_complete` | Truncation completed. |
| `truncate_failed` | Truncation failed. |
| `file_rename_start` | Random rename before deletion started. |
| `file_renamed` | Random rename succeeded. |
| `file_remove_start` | File removal started. |
| `file_removed` | File removed. |
| `benchmark` | Benchmark result for a file. |
| `file_list_not_found` | File-list path was missing. |
| `file_list_start` | File-list processing started. |
| `file_list_line_skipped` | Empty/comment line skipped. Verbose-only. |
| `file_list_path_missing` | Listed target missing. |
| `file_list_path_requires_force` | Listed target skipped because `--force` was absent. |
| `file_list_path_start` | Listed target processing started. |
| `file_list_path_complete` | Listed target processing completed. |
| `file_list_path_failed` | Listed target failed. |
| `file_list_line_read_failed` | Could not read a file-list line. |
| `file_list_complete` | File-list processing completed. |

## Status values

| Status | Success | Exit code | Meaning |
|---|---:|---:|---|
| `completed` | true | `0` | Operation completed. |
| `failed` | false | `1` | Runtime/file operation failed. |
| `usage` | false | `2` | CLI usage error. |
| `cancelled` | false | `3` | Operation was cancelled or refused due to missing confirmation. |

## Wrapper success policy

For strict wrappers, treat the run as successful only when:

```text
exit_code == 0
summary.success == true
summary.status == "completed"
summary.summary.errors == 0
summary.summary.warnings == 0
```

For tolerant wrappers, you may allow warnings, but log them. A warning can indicate that an overwrite pass or truncation failed even if deletion later succeeded.

## JSON parsing example: Python

```python
import json
import subprocess

result = subprocess.run(
    ["shredator", "./secret.txt", "--force", "--json"],
    text=True,
    capture_output=True,
)

try:
    payload = json.loads(result.stdout)
except json.JSONDecodeError as exc:
    raise RuntimeError(f"Shredator did not emit valid JSON: {exc}\nstdout={result.stdout!r}\nstderr={result.stderr!r}")

if result.returncode != payload.get("exit_code"):
    raise RuntimeError("Process exit code and JSON exit_code disagree")

if not payload.get("success"):
    raise RuntimeError(f"Shredator failed: {payload}")

if payload["summary"]["warnings"]:
    raise RuntimeError(f"Shredator completed with warnings: {payload}")
```

## JSONL parsing example: Python

```python
import json
import subprocess

proc = subprocess.Popen(
    ["shredator", "./big-dir", "--force", "--jsonl"],
    text=True,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
)

summary = None
for line in proc.stdout:
    obj = json.loads(line)
    if obj.get("type") == "event":
        print(f"[{obj['level']}] {obj['event']}: {obj['message']}")
    elif obj.get("type") == "summary":
        summary = obj

return_code = proc.wait()
if summary is None:
    raise RuntimeError("No Shredator summary received")
if return_code != summary["exit_code"]:
    raise RuntimeError("Exit code mismatch")
if not summary["success"]:
    raise RuntimeError(summary)
```

## Compatibility notes

For future-compatible parsers:

- Require `schema` on summary objects.
- Ignore unknown fields.
- Treat missing required fields as parser errors.
- Do not parse `message` for logic. Use `event`, `status`, `exit_code`, and typed fields.
- Prefer JSONL for long-running operations.
