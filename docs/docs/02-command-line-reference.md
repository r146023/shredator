# Command-Line Reference

## Usage

```bash
shredator <file_or_directory_path> [options]
shredator --file-list <path> [options]
```

The positional path form processes exactly one file or directory. The `--file-list` form processes multiple paths from a text file.

## Options overview

| Option | Value | Default | Description |
|---|---:|---:|---|
| `-p`, `--passes` | integer | `3` | Number of overwrite passes. |
| `-v`, `--verbose` | none | off | Include extra progress details. |
| `-q`, `--quiet` | none | off | In text mode, suppress normal info output and print errors/final summary. |
| `-f`, `--force` | none | off | Skip confirmation/important-file protections. Required for non-interactive machine workflows that target protected categories. |
| `--pattern` | pattern name | `random` | Overwrite pattern. |
| `--max-depth` | integer | unlimited | Maximum recursion depth for directories. |
| `--include` | pattern | none | Only process matching files. May be repeated. |
| `--exclude` | pattern | none | Skip matching paths. May be repeated. |
| `--benchmark` | none | off | Emit throughput information per file. |
| `--zero-names` | none | off | Rename files to random hexadecimal names before deletion. |
| `--file-list` | path | none | Read target paths from a text file. |
| `--output` | `text`, `json`, `jsonl` | `text` | Choose output mode. |
| `--format` | `text`, `json`, `jsonl` | `text` | Alias for `--output`. |
| `--json` | none | off | Alias for `--output json`. |
| `--machine`, `--machine-readable` | none | off | Alias for `--output json`. |
| `--jsonl`, `--ndjson` | none | off | Alias for `--output jsonl`. |
| `-h`, `--help` | none | off | Print help in text mode or emit a help event in machine mode. |

## `--passes <number>`

Controls how many overwrite passes are attempted before truncation and deletion.

```bash
shredator ./secret.txt --passes 1 --force
shredator ./secret.txt --passes 3 --force
shredator ./secret.txt --passes 7 --force
```

Notes:

- The default is `3`.
- Higher values take longer.
- `--pattern gutmann` forces the pass count to `35`.
- If an overwrite pass fails, Shredator records a warning and continues to truncation/deletion, preserving the legacy behavior.

## `--verbose`

Enables extra progress events.

```bash
shredator ./data --force --verbose
shredator ./data --force --jsonl --verbose
```

In JSON/JSONL mode, verbose-only events are only included when `--verbose` is set.

Verbose is useful for:

- Diagnosing directory removal retries.
- Seeing skipped empty/comment lines in file lists.
- Debugging wrapper behavior.

## `--quiet`

Suppresses normal info output in text mode.

```bash
shredator ./secret.txt --force --quiet
```

Notes:

- Errors are still printed.
- A final summary is still printed.
- In machine-readable modes, `--quiet` has little practical value because structured events are the output contract.

## `--force`

Bypasses confirmation prompts and important-file/directory safeguards.

```bash
shredator ./archive --force
```

Use `--force` in wrappers. Machine-readable modes are non-interactive, so Shredator will not stop to ask for `y/n` confirmation.

Without `--force`, Shredator treats the following as requiring confirmation/protection:

- Directories.
- Files with extensions: `doc`, `docx`, `pdf`, `xls`, `xlsx`, `ppt`, `pptx`, `jpg`, `png`.
- Files larger than 10 MB.

In single-path text mode, the user may be prompted. In machine-readable mode, Shredator reports that confirmation is required and exits with the cancelled status.

## `--pattern <type>`

Selects the overwrite pattern.

Accepted values:

| Value | Meaning |
|---|---|
| `random` | Fill with random bytes. |
| `zeros`, `zero` | Fill with `0x00`. |
| `ones`, `one` | Fill with `0xFF`. |
| `alt`, `alternating` | Alternate between `0xAA` and `0x55` by pass. |
| `dod` | DoD-style cycle: zeros, ones, random. |
| `gutmann` | Gutmann-style 35-pass sequence. |

Examples:

```bash
shredator ./secret.txt --pattern random --force
shredator ./secret.txt --pattern zeros --passes 1 --force
shredator ./secret.txt --pattern dod --passes 3 --force
shredator ./secret.txt --pattern gutmann --force
```

## `--max-depth <number>`

Limits directory recursion depth.

```bash
shredator ./root --force --max-depth 0
shredator ./root --force --max-depth 1
shredator ./root --force --max-depth 3
```

Depth behavior:

- `0`: process files directly inside the root directory, then remove the root if empty.
- `1`: process files in the root and one nested level.
- `2`: root, children, grandchildren.

If a directory is skipped because it exceeds the depth limit, Shredator records a skip event.

## `--include <pattern>`

Only process files whose **file name** matches the pattern. Directories are still traversed so that matching files inside them can be found.

```bash
shredator ./data --force --include "*.tmp"
shredator ./data --force --include "*.tmp" --include "*.bak"
```

The pattern syntax is intentionally simple:

| Pattern | Behavior |
|---|---|
| `*.tmp` | Filename ends with `.tmp`. |
| `secret*` | Filename starts with `secret`. |
| `*draft*` | Filename contains `draft`. |
| `exact.txt` | Filename equals `exact.txt`. |

This is not a full shell glob engine. It does not support character classes, brace expansion, recursive globstars, or path segment matching.

## `--exclude <pattern>`

Skip paths whose **file name** matches the pattern.

```bash
shredator ./data --force --exclude "*.jpg"
shredator ./data --force --exclude "keep*" --exclude "*.iso"
```

Exclude checks happen before include checks. A path matching an exclude pattern is skipped even if it also matches an include pattern.

## `--benchmark`

Emits per-file benchmark information.

```bash
shredator ./large.bin --force --benchmark
shredator ./large.bin --force --benchmark --json
```

Benchmark output includes:

- File size in MiB.
- Elapsed seconds.
- Throughput in MiB/s.
- Completed overwrite passes.

Benchmark throughput is based on completed overwrite passes, not the truncation or deletion phase.

## `--zero-names`

Renames a file to a random 16-character hexadecimal name before deletion.

```bash
shredator ./secret-client-list.xlsx --force --zero-names
```

Despite the flag name, the updated behavior is random name generation, not literal zero filenames. The point is to reduce the chance that the original filename remains visible in the directory entry after deletion.

Notes:

- This does not guarantee the old filename is unrecoverable.
- File names may still exist in logs, shell history, thumbnails, recent file lists, indexes, backups, sync tools, or filesystem metadata.
- The rename is attempted after truncation and before deletion.
- Shredator tries up to 32 random names to avoid collisions.

## `--file-list <path>`

Reads target paths from a text file, one path per line.

```bash
shredator --file-list ./targets.txt --force
```

Rules:

- Empty lines are skipped.
- Lines beginning with `#` are skipped as comments.
- Every other trimmed line is interpreted as a path.
- Missing paths are skipped with a warning.
- Without `--force`, protected/important paths are skipped instead of prompting.

Example `targets.txt`:

```text
# One target per line
./secrets/passwords.txt
./old-export.csv
./tmp/session-cache
```

## `--output <format>` / `--format <format>`

Chooses output mode.

```bash
shredator ./secret.txt --force --output text
shredator ./secret.txt --force --output json
shredator ./secret.txt --force --output jsonl
```

Accepted values:

- `text`
- `human` alias for `text`
- `json`
- `jsonl`
- `ndjson` alias for `jsonl`
- `lines` alias for `jsonl`

## `--json`

Shortcut for:

```bash
--output json
```

Example:

```bash
shredator ./secret.txt --force --json
```

## `--jsonl` / `--ndjson`

Shortcut for:

```bash
--output jsonl
```

Examples:

```bash
shredator ./secret.txt --force --jsonl
shredator ./secret.txt --force --ndjson
```

Use JSONL for progress-aware wrappers because each event is emitted as soon as it occurs.

## `--machine-readable`

Shortcut for:

```bash
--output json
```

Example:

```bash
shredator ./secret.txt --force --machine-readable
```

## Unknown options

Unknown options produce a usage error and exit code `2`.

```bash
shredator ./secret.txt --definitely-not-a-real-flag
```

## Multiple positional paths

The updated argument parser accepts only one positional path. To process multiple paths, use `--file-list`.

```bash
# Wrong
shredator ./a.txt ./b.txt --force

# Right
printf "./a.txt\n./b.txt\n" > targets.txt
shredator --file-list targets.txt --force
```
