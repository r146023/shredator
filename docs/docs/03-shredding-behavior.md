# Shredding Behavior

This document explains what Shredator actually does at runtime.

## Single-file processing

For a regular file with non-zero size:

1. Read file metadata.
2. Record the file size in the summary as `bytes_seen`.
3. Emit `file_start`.
4. For each pass:
   - Emit `overwrite_pass_start`.
   - Open the file for writing.
   - Seek to byte `0`.
   - Write the selected pattern across the original file size.
   - Flush and sync the file.
   - Emit `overwrite_pass_complete`.
5. Truncate the file to zero bytes.
6. Optionally rename the file if `--zero-names` is set.
7. Remove the file.
8. Emit `file_removed`.
9. Increment summary counters.

## Empty files

If the file size is zero, Shredator does not attempt overwrite passes. It removes the file directly.

Events:

- `file_start`
- `empty_file_remove`
- `file_removed`

This is intentional. There is no content region to overwrite.

## Directory processing

For a directory, Shredator:

1. Reads the directory entries.
2. Processes children first.
3. Recurses into child directories unless the depth limit is exceeded.
4. Applies exclude filters before include filters.
5. Shreds files that survive filtering.
6. Removes the directory after its contents are processed.
7. Retries directory removal a few times if the OS temporarily refuses removal.

Directory removal retries are especially useful on Windows, where recently closed file handles can occasionally linger for a short moment.

## Directory depth

Depth is counted from the root directory you pass to Shredator.

```text
root/                  depth 0
root/file.txt          file inside depth 0
root/a/                depth 1
root/a/file.txt        file inside depth 1
root/a/b/              depth 2
root/a/b/file.txt      file inside depth 2
```

Examples:

```bash
# Process only files directly inside root
shredator ./root --force --max-depth 0

# Process root and one nested directory level
shredator ./root --force --max-depth 1
```

A directory skipped by depth limit is not removed.

## Include and exclude filters

Filters match only the final filename, not the full path.

Given:

```text
archive/
  keep/report.pdf
  tmp/report.tmp
  tmp/cache.bin
```

Command:

```bash
shredator ./archive --force --include "*.tmp"
```

Behavior:

- Directories are still traversed.
- `report.tmp` is processed.
- `report.pdf` is skipped.
- `cache.bin` is skipped.
- Directories may fail to remove if skipped files remain inside them.

### Exclude wins over include

```bash
shredator ./archive --force --include "*.tmp" --exclude "secret*"
```

A file named `secret.tmp` is skipped because exclude is checked first.

## File-list processing

`--file-list` reads a line-based target file:

```text
# comments are skipped
./a.txt
./b.txt

./old-dir
```

Rules:

- Empty lines are skipped.
- Comment lines beginning with `#` are skipped.
- Missing targets increment `paths_skipped` and emit warnings.
- In file-list mode, protected targets without `--force` are skipped instead of prompting.
- The whole run fails if any processed path fails.

## Confirmation behavior

Shredator has a protective confirmation system for high-risk operations.

A path requires confirmation when:

- It is a directory.
- It has one of these extensions: `doc`, `docx`, `pdf`, `xls`, `xlsx`, `ppt`, `pptx`, `jpg`, `png`.
- It is larger than 10 MB.

### Text mode, single path

Without `--force`, Shredator prompts:

```text
Warning: You are about to permanently destroy ./archive
This operation cannot be undone. Continue? (y/n)
```

Only `y` continues. Anything else cancels.

### Machine-readable mode

Machine-readable modes never prompt. If a prompt would be required, Shredator emits `confirmation_required`, marks the path skipped, and exits with the cancelled status.

Wrapper rule: always use `--force` when you already have user authorization.

## Failure behavior

### Overwrite pass failure

If a pass fails, Shredator records `overwrite_pass_failed`, then continues to truncation/deletion.

That behavior is inherited from the original app and is important to understand. A wrapper should inspect `summary.warnings` and the event stream, not only the final exit code, if it needs high confidence about every overwrite pass.

### Truncate failure

If truncation fails, Shredator records `truncate_failed` and still attempts deletion.

### Delete failure

If the final remove operation fails, the path operation fails.

### Directory removal failure

If directory removal fails after retries, the operation fails.

## Summary counters

| Counter | Meaning |
|---|---|
| `files_shredded` | Number of files removed. Empty files count after successful removal. |
| `directories_removed` | Number of directories removed. |
| `paths_successful` | Top-level file-list paths or the single requested path that completed successfully. |
| `paths_failed` | Top-level paths that failed. |
| `paths_skipped` | Skipped paths, missing file-list paths, depth skips, excluded paths, non-included paths, etc. |
| `bytes_seen` | Sum of original sizes of files that entered `shred_file`. |
| `bytes_overwritten` | Sum of bytes successfully written across overwrite passes. |
| `overwrite_passes_completed` | Total completed overwrite passes across all files. |
| `warnings` | Number of warning events. |
| `errors` | Number of error events. |

## What counts as successful deletion?

At the process level, success means Shredator completed its file and directory operations without an unrecovered I/O error. It does not mean forensic unrecoverability is guaranteed.

For wrappers, a high-confidence success check should require:

- Exit code `0`.
- Final JSON/JSONL summary `success: true`.
- `status: "completed"`.
- `summary.errors == 0`.
- Usually `summary.warnings == 0`, unless your wrapper explicitly tolerates warnings.
- For a single file, `files_shredded == 1`.
- For file-list runs, `paths_failed == 0`.
