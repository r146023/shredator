# Troubleshooting

## `Error: Path does not exist`

The target path was not found.

Fixes:

- Use an absolute path.
- Check quoting.
- Check current working directory.
- Check whether the file was already deleted.

Example:

```bash
pwd
ls -la ./secret.txt
shredator "$(pwd)/secret.txt" --force --json
```

## `Unknown option`

The flag is not supported or was misspelled.

Check:

```bash
shredator --help
```

Common mistakes:

```bash
# Wrong
shredator ./secret.txt --pass 3

# Right
shredator ./secret.txt --passes 3
```

## `--passes option requires a value`

You supplied `--passes` without a number.

```bash
# Wrong
shredator ./secret.txt --passes --force

# Right
shredator ./secret.txt --passes 3 --force
```

## `Invalid number of passes`

The pass value must be an unsigned integer.

```bash
# Wrong
shredator ./secret.txt --passes many --force

# Right
shredator ./secret.txt --passes 3 --force
```

## Machine mode exits cancelled

Possible cause: target requires confirmation and `--force` was not supplied.

```bash
# May cancel in machine mode
shredator ./important.pdf --json

# Intended wrapper form
shredator ./important.pdf --force --json
```

## Directory removal fails

Possible causes:

- Excluded files remain in the directory.
- Non-included files remain in the directory.
- Another process created a new file while Shredator was running.
- A file handle is still open.
- Permission denied.

Example:

```bash
shredator ./dir --force --include "*.tmp"
```

If non-`.tmp` files remain, removing `./dir` can fail because the directory is not empty.

Fixes:

- Do not expect directory removal when using include/exclude filters that leave files behind.
- Close programs using the directory.
- Re-run without restrictive filters if you intend to remove the entire directory.

## File is locked on Windows

Close any program that may have the file open.

Common culprits:

- File Explorer preview pane.
- Antivirus scanner.
- Editor/IDE.
- Media player.
- Indexing service.

Retry after closing the handle owner.

## Permission denied

Fixes:

- Run from an account that owns the file.
- Check file permissions.
- Check directory permissions.
- Avoid protected OS directories.
- Avoid shredding files owned by another process/user unless intended.

## JSON parser fails

Possible causes:

- You used text output instead of `--json` or `--jsonl`.
- The binary crashed before emitting JSON.
- Your wrapper mixed stderr into stdout.
- You are reading only part of stdout.

Fix:

```bash
shredator ./secret.txt --force --json > result.json 2> error.log
jq . result.json
```

In wrappers, capture stdout and stderr separately.

## JSONL parser fails on the final line

The final JSONL line is a summary object, not an event object. Handle both:

```python
if obj.get("type") == "event":
    handle_event(obj)
elif obj.get("type") == "summary":
    handle_summary(obj)
```

## Warnings but exit code 0

Warnings can happen when an overwrite pass or truncation fails but deletion succeeds. The process can still report completed depending on the exact path behavior.

Strict wrappers should treat warnings as failure:

```text
summary.warnings == 0
```

## `--zero-names` failed

Possible causes:

- No permission to rename in the parent directory.
- Parent directory is read-only.
- File is locked.
- Extremely unlikely random-name collision 32 times.

Fixes:

- Check parent directory permissions.
- Close processes using the file.
- Retry.

## Benchmark throughput looks low

Possible causes:

- Random generation overhead.
- Slow disk.
- Antivirus scanning writes.
- Sync tool watching the directory.
- Running on network storage.
- High pass count.
- File sync/flush cost after each pass.

Use `--pattern zeros --passes 1 --benchmark` as a baseline.

## File-list skipped too much

Remember:

- Empty lines are skipped.
- Lines beginning with `#` are skipped.
- Missing paths are skipped.
- Important/protected paths are skipped without `--force`.

Use:

```bash
shredator --file-list targets.txt --force --verbose --jsonl
```

## Include/exclude patterns do not match full paths

Patterns are matched against the final filename only.

```text
/path/to/client-secret/report.txt
```

The filename is:

```text
report.txt
```

So this will not match:

```bash
--include "*client-secret*"
```

Generate a file list externally when you need full-path filtering.
