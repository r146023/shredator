# Shredator: Secure File & Directory Shredder

[![License](https://img.shields.io/badge/license-see%20LICENSE-blue.svg)](./LICENSE)
[![Rust Version](https://img.shields.io/badge/rust-1.64%2B-blue.svg)](https://www.rust-lang.org/)

Shredator is a cross-platform command-line utility for best-effort secure file and directory deletion. It overwrites file contents, optionally renames files before deletion, supports recursive directory processing, can process path lists, and can emit either human-readable logs or machine-readable JSON Lines for automation.

Shredator is designed to be practical, scriptable, and careful. It can make recovery from ordinary filesystem deletion much harder, especially on traditional magnetic hard drives. However, no file-level shredder can honestly guarantee permanent destruction on every storage medium. SSDs, flash media, journaling filesystems, copy-on-write filesystems, snapshots, cloud sync folders, backups, and wear-leveling behavior can leave stale copies outside the control of any normal file-overwrite tool.

For high-assurance sanitization of SSDs or decommissioned media, use full-disk encryption before sensitive data is written, drive-level sanitize / secure erase / cryptographic erase commands, or physical destruction where appropriate.

---

## Table of Contents

- [Features](#features)
- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Command Line Options](#command-line-options)
- [Machine-Readable Output](#machine-readable-output)
- [Shredding Patterns](#shredding-patterns)
- [Batch Processing](#batch-processing)
- [Examples](#examples)
- [Recipes](#recipes)
- [Exit Codes](#exit-codes)
- [Security Considerations](#security-considerations)
- [Technical Details](#technical-details)
- [Contributing](#contributing)
- [License](#license)
- [Feedback and Support](#feedback-and-support)

---

## Features

- **Multiple Overwrite Patterns**: Choose from random data, zero-fill, one-fill, alternating bits, DoD-style passes, or Gutmann-style wiping.
- **Recursive Directory Shredding**: Process entire directory trees with optional maximum recursion depth.
- **Configurable Passes**: Balance speed and overwrite intensity with a configurable pass count.
- **File Filtering**: Include or exclude files using glob-style pattern matching.
- **Safety Confirmations**: Confirmation prompts for sensitive or important files unless `--force` is used.
- **Batch Processing**: Process many files or directories from a newline-delimited path list.
- **Zero-Name Security**: Optionally rename files to random names before deletion to reduce filename leakage.
- **Machine-Readable Responses**: Emit JSON Lines for integration with scripts, wrappers, GUIs, and external scripts, wrappers, GUIs, and automation tools.
- **Human-Readable Reports**: Emit robocopy-style per-file logs and a final summary by default.
- **Performance Benchmarking**: Measure and report shredding speed and throughput.
- **Cross-Platform**: Designed to run on Linux, macOS, and Windows.
- **Continue-on-Error Behavior**: Batch and directory runs continue after per-path failures and report failures in the final summary.

---

## Installation

### From Source

1. Ensure Rust and Cargo are installed:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

2. Clone the repository:

```bash
git clone https://github.com/r146023/shredator.git
cd shredator
```

3. Build the project:

```bash
cargo build --release
```

4. The executable will be available in:

```text
target/release/shredator
```

On Windows, the executable will be:

```text
target\release\shredator.exe
```

### Using Cargo from Git

If you want to install directly from the public repository:

```bash
cargo install --git https://github.com/r146023/shredator.git
```

If Shredator is later published to crates.io, it can also be installed with:

```bash
cargo install shredator
```

---

## Basic Usage

```bash
shredator <file_or_directory_path> [options]
```

Examples:

```bash
shredator sensitive_document.pdf
shredator ./old_private_folder --force
shredator ./logs --include "*.log" --passes 1
```

By default, Shredator overwrites matching files, flushes changes where supported, truncates files, optionally renames them if requested, and then removes them from the filesystem.

When processing directories, Shredator walks the directory tree recursively unless limited by `--max-depth` or by the options supported by your build.

---

## Command Line Options

| Option | Description |
|--------|-------------|
| `<file_or_directory_path>` | File or directory to shred. Omit when using `--file-list` if supported by your build. |
| `-p, --passes <number>` | Number of overwrite passes. Default: `3`. |
| `-v, --verbose` | Display detailed progress information. |
| `-q, --quiet` | Only display errors and final summary. |
| `-f, --force` | Skip confirmation prompts for sensitive operations. |
| `--pattern <type>` | Overwrite pattern: `random`, `zeros`, `ones`, `alt`, `dod`, or `gutmann`. |
| `--max-depth <number>` | Maximum directory recursion depth. |
| `--include <pattern>` | Only process files matching the pattern, for example `*.txt`. May be repeated if supported by your build. |
| `--exclude <pattern>` | Skip files matching the pattern, for example `*.jpg`. May be repeated if supported by your build. |
| `--benchmark` | Measure and report performance statistics. |
| `--zero-names` | Rename files to random names before deletion. |
| `--file-list <path>` | Read paths to shred from a text file, one path per line. |
| `--machine-responses` | Emit machine-readable JSON Lines instead of human-readable logs. |
| `--machine_responses` | Alias for `--machine-responses`, useful for tools that prefer underscore-style flags. |
| `--dry-run` | Simulate what would be shredded without modifying files, if supported by your build. |
| `--dry_run` | Alias for `--dry-run`. |
| `-h, --help` | Show help. |
| `--version` | Show version information. |

> Note: Some options may depend on the exact build/version of Shredator. Run `shredator --help` to see the options compiled into your local executable.

---

## Machine-Readable Output

Shredator supports machine-readable output for automation and wrappers. Enable it with:

```bash
shredator ./private --force --machine-responses
```

or:

```bash
shredator ./private --force --machine_responses true
```

Machine-readable mode emits **JSON Lines**: one JSON object per line. This is better for long-running operations than a single giant JSON document because callers can parse progress incrementally.

### Event Types

Typical events include:

```json
{"type":"start","target":"./private","mode":"shred","passes":3,"pattern":"random"}
{"type":"file_start","path":"./private/a.txt","size_bytes":12345}
{"type":"file_success","path":"./private/a.txt","passes":3,"bytes_processed":37035,"deleted":true,"duration_ms":28}
{"type":"file_warning","path":"./private/missing.txt","code":"not_found","message":"Path does not exist; skipped."}
{"type":"file_error","path":"./private/locked.db","code":"permission_denied","message":"Could not open file for writing."}
{"type":"summary","files_seen":10,"files_shredded":8,"files_failed":1,"files_skipped":1,"duration_ms":944}
```

### Recommended Parser Behavior

Automation code should:

1. Read stdout line-by-line.
2. Parse each line as JSON.
3. Treat `file_success` as proof that Shredator completed its requested file-level operation for that path.
4. Treat `file_warning` as non-fatal unless your workflow requires strict success.
5. Treat `file_error` as a per-file failure.
6. Use the final `summary` event to decide whether the whole run succeeded, partially succeeded, or failed.

### Machine Output Stability

Machine-readable responses are intended to be stable enough for integration. Future versions may add fields, but should avoid renaming or removing existing fields without a version bump.

Recommended consumer rule:

```text
Ignore fields you do not understand.
Require only the fields your integration actually needs.
```

### Human Logs vs Machine Logs

Human-readable mode is optimized for terminal use:

```text
SHRED     ./private/a.txt
PASS 1/3  random  12.1 KiB
PASS 2/3  random  12.1 KiB
PASS 3/3  random  12.1 KiB
DELETE    ./private/a.txt
OK        ./private/a.txt
```

Machine-readable mode is optimized for callers:

```json
{"type":"file_success","path":"./private/a.txt","passes":3,"deleted":true}
```

Do not scrape human-readable logs in automation. Use `--machine-responses`.

---

## Shredding Patterns

Shredator supports several overwrite patterns. Each pattern has different speed, compatibility, and security characteristics.

### Random

Overwrites the file with random data for the configured number of passes.

```bash
shredator secrets.txt --pattern random --passes 3
```

This is the recommended default for general use on traditional magnetic drives.

### Zeros

Overwrites the file with zero bytes, `0x00`.

```bash
shredator scratch.dat --pattern zeros --passes 1
```

This is fast and useful when you want a simple wipe pattern. It is not meaningfully better than random for SSDs because SSD behavior is controlled by the firmware and flash translation layer.

### Ones

Overwrites the file with one bytes, `0xFF`.

```bash
shredator scratch.dat --pattern ones --passes 1
```

### Alternating (`alt`)

Alternates between bit patterns such as `0x55` and `0xAA`.

```bash
shredator old_keys.bin --pattern alt --passes 2
```

### DoD 5220.22-M Style (`dod`)

Uses a three-pattern sequence:

1. Pass 1: zero bytes, `0x00`
2. Pass 2: one bytes, `0xFF`
3. Pass 3: random data

```bash
shredator tax_returns.zip --pattern dod
```

This pattern is included for compatibility with familiar workflows. More passes are not automatically better on modern media, especially SSDs.

### Gutmann Style (`gutmann`)

Uses a 35-pass Gutmann-style pattern.

```bash
shredator archive.img --pattern gutmann
```

This is slow and usually unnecessary for modern drives. It is provided for users who explicitly want that pattern, not because it is the best default.

---

## Batch Processing

Shredator can process multiple files and directories listed in a text file:

```bash
shredator --file-list paths_to_shred.txt --force
```

Example list file:

```text
# Comments start with #
/path/to/file1.txt
/path/to/file2.pdf
/path/to/directory

# Blank lines are ignored
```

Batch behavior:

- Lines beginning with `#` are ignored.
- Empty lines are skipped.
- Missing paths are skipped with a warning.
- Per-path failures are logged.
- Processing continues after failures.
- The final summary reports successful, failed, and skipped paths.

Machine-readable batch example:

```bash
shredator --file-list paths_to_shred.txt --force --machine-responses
```

Example output:

```json
{"type":"start","source":"file_list","file_list":"paths_to_shred.txt"}
{"type":"file_success","path":"/path/to/file1.txt","deleted":true}
{"type":"file_warning","path":"/path/to/missing.txt","code":"not_found","message":"Path does not exist; skipped."}
{"type":"summary","files_seen":3,"files_shredded":1,"files_failed":0,"files_skipped":1}
```

---

## Examples

### Basic File Shredding

```bash
# Shred a single file with default settings
shredator sensitive_document.pdf

# Shred with 7 random passes
shredator financial_data.xlsx --passes 7
```

### Directory Shredding

```bash
# Recursively shred a directory and all its contents
shredator ~/old_projects/confidential/

# Limit recursion depth
shredator ~/logs/ --max-depth 2
```

### Pattern Selection

```bash
# Use DoD-style pattern
shredator ~/tax_returns/ --pattern dod

# Use Gutmann-style 35-pass wipe
shredator ~/crypto_keys/ --pattern gutmann
```

### File Filtering

```bash
# Only shred text files
shredator ~/documents/ --include "*.txt"

# Shred everything except images
shredator ~/documents/ --exclude "*.jpg" --exclude "*.png"
```

### Performance Benchmarking

```bash
# Measure shredding performance
shredator large_file.dat --benchmark
```

### Enhanced Filename Privacy

```bash
# Rename files to random names before deletion
shredator ~/private/ --zero-names
```

### Machine-Readable Automation

```bash
# Emit JSON Lines for wrappers, GUI tools, or automation
shredator ~/private/ --force --machine-responses
```

### Dry Run

```bash
# Show what would be shredded without deleting anything
shredator ~/private/ --dry-run --verbose
```

---

## Recipes

### Shred a Single File Safely

```bash
shredator ./secret.txt --verbose
```

Use this when you want readable progress and confirmation prompts.

### Shred a Single File Without Prompts

```bash
shredator ./secret.txt --force
```

Use this in scripts only when you are certain the target path is correct.

### Shred a Directory but Keep the Run Bounded

```bash
shredator ./old_exports --max-depth 3 --force
```

This prevents accidentally walking deeper than expected.

### Shred Only Temporary Files

```bash
shredator ./workdir --include "*.tmp" --include "*.bak" --force
```

Useful for build folders, export folders, and temporary working directories.

### Exclude Media Files

```bash
shredator ./archive --exclude "*.jpg" --exclude "*.png" --exclude "*.mp4" --force
```

Useful when you want to shred documents but preserve large media files.

### Generate a Machine-Readable Report

```bash
shredator ./private --force --machine-responses > shredator-report.jsonl
```

Then inspect failures:

```bash
cat shredator-report.jsonl | grep '"type":"file_error"'
```

### Use from Python

```python
import json
import subprocess

cmd = [
    "shredator",
    "./private",
    "--force",
    "--machine-responses",
]

proc = subprocess.Popen(
    cmd,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE,
    text=True,
)

for line in proc.stdout:
    event = json.loads(line)
    if event.get("type") == "file_success":
        print("shredded", event.get("path"))
    elif event.get("type") == "file_error":
        print("failed", event.get("path"), event.get("message"))

exit_code = proc.wait()
if exit_code != 0:
    print("Shredator completed with errors")
```

### Use from PowerShell

```powershell
shredator.exe .\private --force --machine-responses | ForEach-Object {
    $event = $_ | ConvertFrom-Json
    if ($event.type -eq "file_success") {
        Write-Host "Shredded: $($event.path)"
    }
    elseif ($event.type -eq "file_error") {
        Write-Host "Failed: $($event.path) - $($event.message)"
    }
}
```

---

## Exit Codes

Recommended exit-code behavior:

| Code | Meaning |
|------|---------|
| `0` | Completed successfully with no file failures. |
| `1` | Completed with one or more file-level failures or skipped required paths. |
| `2` | Invalid command-line arguments. |
| `3` | Fatal setup error, such as unreadable file list or inaccessible target root. |
| `4` | Interrupted or cancelled. |
| `5` | Unexpected internal error. |

For automation, prefer machine-readable `summary` data over exit code alone. The exit code tells you whether the process as a whole was clean; the JSONL events tell you exactly what happened per path.

---

## Security Considerations

### Shredator Is Best-Effort File-Level Deletion

Shredator can overwrite and delete files through normal filesystem APIs. That is useful, but it is not the same as whole-device sanitization.

Shredator can report:

```text
The target file was overwritten according to the selected pattern, flushed where supported, truncated, optionally renamed, and deleted.
```

Shredator should not be interpreted as proving:

```text
No recoverable copy of this data exists anywhere on this SSD, filesystem journal, backup, snapshot, cloud sync folder, or remapped block.
```

That stronger claim is outside the control of a file-level utility.

### Physical Media Types

#### Hard Disk Drives

Shredator is most meaningful on traditional magnetic hard drives where overwriting a logical file is more likely to overwrite the physical sectors that held the data.

#### Solid State Drives

SSDs use wear leveling, remapping, spare blocks, TRIM behavior, and firmware-managed flash translation layers. A file-level overwrite may not overwrite the physical cells that previously contained the file. For SSDs, use:

- Full-disk encryption before sensitive data is written.
- Drive-level sanitize / secure erase / cryptographic erase where available.
- Physical destruction for media that must never be recoverable and will leave your control.

#### USB Drives and SD Cards

Flash media has many of the same problems as SSDs, often with less transparent firmware behavior. Treat file-level shredding as best-effort only.

#### Copy-on-Write Filesystems

Filesystems such as Btrfs, ZFS, APFS, and other snapshotting or copy-on-write systems may retain old versions of file blocks. Shredding the current file path may not erase prior snapshots or retained extents.

#### Cloud-Synced Folders

Cloud sync tools may upload old versions, keep revision history, cache files locally, or replicate data to other devices. Shredator only operates on the local path it is given.

### Metadata Leakage

Even when file contents are overwritten, metadata may remain elsewhere:

- filenames
- directory names
- recently opened files lists
- thumbnails
- search indexes
- application temp files
- filesystem journals
- backups
- shell history

Use `--zero-names` to reduce filename leakage at the target path, but understand that it cannot clean every external metadata source.

### Recommended High-Assurance Workflow

For sensitive data that already exists on an SSD:

1. Move the data you still need into an encrypted container or encrypted backup.
2. Verify the encrypted copy.
3. Sanitize the entire original drive using device-level sanitize / secure erase / cryptographic erase.
4. Restore only the encrypted copy.

For future data:

1. Enable full-disk encryption first.
2. Store sensitive working files only on encrypted volumes.
3. Use Shredator for best-effort cleanup of local files.
4. Use drive-level sanitization when decommissioning media.

---

## Technical Details

### Buffer Sizes

Shredator uses a fixed-size buffer for overwrite passes. The default is intended to provide a reasonable balance between memory usage and throughput across common storage devices.

### Write / Flush / Sync Behavior

Shredator attempts to write overwrite data through normal file APIs and flush changes where supported by the platform. Exact durability behavior depends on:

- operating system
- filesystem
- drive firmware
- storage controller
- mount options
- hardware cache behavior

### Deletion Flow

A typical file shredding flow is:

```text
open file for writing
record file size
for each configured pass:
    seek to start
    overwrite file contents
    flush/sync where supported
truncate file
optional random rename
remove file
log result
```

If a step fails, Shredator logs the failure and reports that file as failed.

### Important File Detection

Files may be considered important and require confirmation if they:

- have extensions associated with documents, spreadsheets, presentations, archives, keys, databases, or images
- are larger than a configured threshold
- are located in sensitive-looking folders

Use `--force` to skip prompts when running intentionally in scripts.

### Directory Processing

When a directory is provided, Shredator walks files first and then removes empty directories where appropriate. If files fail to shred, parent directory deletion may also fail or be skipped.

### File List Processing

When `--file-list` is used, Shredator treats each non-empty, non-comment line as a path. Directories in the list are processed using the same directory traversal rules as direct directory targets.

### Machine Response Design

JSONL events are emitted in chronological order. For parallel or future threaded builds, events may be interleaved between files. Consumers should use the `path` field or any available operation ID rather than assuming all events for a file are contiguous.

---

## Contributing

Contributions are welcome. Please open an issue for bugs, feature requests, or design discussion before submitting larger changes.

General workflow:

1. Fork the repository.
2. Create your feature branch:

```bash
git checkout -b feature/amazing-feature
```

3. Commit your changes:

```bash
git commit -m "Add some amazing feature"
```

4. Push to the branch:

```bash
git push origin feature/amazing-feature
```

5. Open a Pull Request.

Before submitting changes, run:

```bash
cargo fmt
cargo clippy
cargo test
cargo build --release
```

---

## License

See the `LICENSE` file located next to this `README.md` for the project license.

---

## Feedback and Support

For bug reports, feature requests, or technical support, contact:

```text
ApolithSynthetic@gmail.com
```

---

## Responsible Use

Use Shredator responsibly. Always ensure you have proper authorization before deleting data, especially in shared, workplace, legal, or regulated environments.

Before shredding important data, confirm that:

- you selected the correct path
- you have backups of anything you still need
- you understand the limits of file-level deletion on your storage medium
- you are authorized to delete the data

