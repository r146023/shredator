# colemen_py Wrapper Contract

This document proposes a clean wrapper contract for integrating Shredator into `colemen_py`.

## Goals

The wrapper should make `colemen_py` confident that Shredator finished before moving on.

That means the wrapper should not merely spawn the process. It should:

- Wait for completion.
- Parse machine-readable output.
- Compare process exit code with JSON `exit_code`.
- Check `success` and `status`.
- Inspect warnings/errors.
- Return a typed result object.
- Raise or return structured failure when deletion cannot be confirmed.

## Recommended command

For normal single-target operations:

```text
shredator <path> --force --output json
```

For progress-aware operations:

```text
shredator <path> --force --output jsonl
```

For many targets:

```text
shredator --file-list <generated-list-path> --force --output jsonl
```

## Wrapper result type

Suggested Python model:

```python
from dataclasses import dataclass
from pathlib import Path
from typing import Any

@dataclass(frozen=True)
class ShredatorSummary:
    files_shredded: int
    directories_removed: int
    paths_successful: int
    paths_failed: int
    paths_skipped: int
    bytes_seen: int
    bytes_overwritten: int
    overwrite_passes_completed: int
    warnings: int
    errors: int

@dataclass(frozen=True)
class ShredatorRunResult:
    binary: Path
    target: Path | None
    file_list: Path | None
    returncode: int
    schema: str
    success: bool
    status: str
    exit_code: int
    duration_ms: int
    output_format: str
    summary: ShredatorSummary
    events: list[dict[str, Any]]
    stderr: str
```

## Strict success method

```python
def is_strict_success(result: ShredatorRunResult) -> bool:
    return (
        result.returncode == 0
        and result.exit_code == 0
        and result.success is True
        and result.status == "completed"
        and result.summary.errors == 0
        and result.summary.warnings == 0
    )
```

## Failure classes

Suggested exceptions:

```python
class ShredatorError(RuntimeError): ...
class ShredatorBinaryNotFound(ShredatorError): ...
class ShredatorInvalidOutput(ShredatorError): ...
class ShredatorExitCodeMismatch(ShredatorError): ...
class ShredatorFailed(ShredatorError): ...
class ShredatorCompletedWithWarnings(ShredatorError): ...
class ShredatorCancelled(ShredatorError): ...
class ShredatorUsageError(ShredatorError): ...
class ShredatorTimeout(ShredatorError): ...
```

## Preflight checks

Before invoking Shredator, `colemen_py` should consider checking:

- Binary exists.
- Binary is executable.
- Target exists.
- Target is inside an allowed root if the API is sandboxed.
- Target is a regular file or directory.
- Whether symlinks are allowed.
- Whether the caller has permission to delete.
- Whether the operation has explicit user approval.

## Path safety

Never build command strings manually.

Good:

```python
subprocess.run([str(binary), str(target), "--force", "--json"])
```

Bad:

```python
subprocess.run(f"{binary} {target} --force --json", shell=True)
```

## Handling file-list mode

If `colemen_py` generates a file list:

1. Store it in a private temp directory.
2. Write absolute paths.
3. Avoid logging contents unless debug mode explicitly allows it.
4. Run Shredator with `--file-list`.
5. Delete or shred the file list after use if it contains sensitive paths.

Example:

```python
with tempfile.NamedTemporaryFile("w", delete=False, encoding="utf-8") as f:
    for path in targets:
        f.write(str(path.resolve()))
        f.write("\n")
    list_path = Path(f.name)

try:
    result = run_shredator_file_list(list_path)
finally:
    if list_path.exists():
        list_path.unlink()
```

## Warning policy

Recommended default: warnings are failures.

Why: Shredator may warn that an overwrite pass or truncation failed but still remove the file. If the higher-level API promises confidence, it should not silently accept that.

Optional policy enum:

```python
class ShredatorWarningPolicy(Enum):
    STRICT = "strict"      # warnings fail
    TOLERANT = "tolerant"  # warnings logged, result returned
```

## Progress callbacks

For JSONL mode:

```python
def on_event(event: dict[str, Any]) -> None:
    name = event.get("event")
    if name == "overwrite_pass_complete":
        update_progress(event["path"], event["pass"], event["total_passes"])
    elif event.get("level") in {"warning", "error"}:
        log_event(event)
```

## Timeout policy

Do not use one tiny global timeout.

Suggested configuration:

```python
@dataclass(frozen=True)
class ShredatorTimeoutPolicy:
    base_seconds: float = 30.0
    min_bytes_per_second: float = 10 * 1024 * 1024
    max_seconds: float | None = None
```

Estimate:

```python
estimated_seconds = base + (total_bytes * passes / min_bytes_per_second)
```

## Log hygiene

Sensitive filenames can be sensitive. Consider redaction modes:

```python
class ShredatorPathLogging(Enum):
    FULL = "full"
    BASENAME = "basename"
    HASHED = "hashed"
    NONE = "none"
```

Store raw machine output only when debug/audit policy allows it.

## Recommended public API

```python
def shred_file(
    path: PathLike,
    *,
    passes: int = 3,
    pattern: str = "random",
    zero_names: bool = False,
    warning_policy: ShredatorWarningPolicy = ShredatorWarningPolicy.STRICT,
) -> ShredatorRunResult: ...


def shred_directory(
    path: PathLike,
    *,
    passes: int = 3,
    pattern: str = "random",
    max_depth: int | None = None,
    include: Sequence[str] = (),
    exclude: Sequence[str] = (),
    zero_names: bool = False,
    progress: Callable[[dict[str, Any]], None] | None = None,
) -> ShredatorRunResult: ...


def shred_paths(
    paths: Sequence[PathLike],
    *,
    passes: int = 3,
    pattern: str = "random",
    progress: Callable[[dict[str, Any]], None] | None = None,
) -> ShredatorRunResult: ...
```

## Suggested result interpretation

| Case | Wrapper behavior |
|---|---|
| `completed`, no warnings/errors | Return success. |
| `completed`, warnings > 0 | Raise by default or return warning result if tolerant. |
| `failed` | Raise `ShredatorFailed`. |
| `usage` | Raise `ShredatorUsageError`, likely wrapper bug. |
| `cancelled` | Raise `ShredatorCancelled`, likely missing `--force` or user cancelled. |
| invalid JSON | Raise `ShredatorInvalidOutput`. |
| process timeout | Kill process, raise `ShredatorTimeout`, mark deletion as unconfirmed. |

## Minimum viable wrapper checklist

- [ ] Finds bundled `shredator` binary.
- [ ] Uses argument arrays.
- [ ] Always uses `--json` or `--jsonl`.
- [ ] Always passes `--force` only after authorization.
- [ ] Parses output.
- [ ] Checks process return code.
- [ ] Checks JSON `exit_code`.
- [ ] Checks `success` and `status`.
- [ ] Checks warnings/errors.
- [ ] Returns typed result or raises typed exception.
