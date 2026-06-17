# Integration Guide

This guide is for wrapping Shredator from another application.

## Integration principles

A wrapper should:

1. Resolve the Shredator binary path explicitly.
2. Pass arguments as an array, not a shell-concatenated string.
3. Use `--force` only after the user has already approved the destructive action.
4. Use `--json` for simple calls or `--jsonl` for progress streaming.
5. Validate the process exit code against the machine-readable `exit_code` field.
6. Treat warnings as important.
7. Never parse human text output for logic.
8. Log the full summary object for audit/debugging.
9. Avoid placing sensitive paths in long-lived logs.

## Recommended command shape

Single target:

```text
shredator <path> --force --output json
```

Long-running target:

```text
shredator <path> --force --output jsonl
```

File list:

```text
shredator --file-list <list-path> --force --output jsonl
```

## Python wrapper: JSON mode

```python
from __future__ import annotations

import json
import subprocess
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class ShredatorResult:
    returncode: int
    payload: dict[str, Any]
    stderr: str

    @property
    def clean_success(self) -> bool:
        summary = self.payload.get("summary", {})
        return (
            self.returncode == 0
            and self.payload.get("success") is True
            and self.payload.get("status") == "completed"
            and summary.get("errors") == 0
            and summary.get("warnings") == 0
        )


def shred_path(binary: Path, target: Path, *, passes: int = 3) -> ShredatorResult:
    cmd = [
        str(binary),
        str(target),
        "--force",
        "--passes",
        str(passes),
        "--output",
        "json",
    ]

    proc = subprocess.run(cmd, text=True, capture_output=True)

    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        raise RuntimeError(
            f"Shredator did not emit valid JSON: {exc}\n"
            f"returncode={proc.returncode}\n"
            f"stdout={proc.stdout!r}\n"
            f"stderr={proc.stderr!r}"
        ) from exc

    if payload.get("exit_code") != proc.returncode:
        raise RuntimeError(
            f"Exit code mismatch: process={proc.returncode}, json={payload.get('exit_code')}"
        )

    return ShredatorResult(proc.returncode, payload, proc.stderr)
```

## Python wrapper: JSONL mode with progress callback

```python
from __future__ import annotations

import json
import subprocess
from pathlib import Path
from typing import Any, Callable


def shred_path_streaming(
    binary: Path,
    target: Path,
    on_event: Callable[[dict[str, Any]], None],
) -> dict[str, Any]:
    proc = subprocess.Popen(
        [str(binary), str(target), "--force", "--output", "jsonl"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    assert proc.stdout is not None
    summary: dict[str, Any] | None = None

    for raw_line in proc.stdout:
        line = raw_line.strip()
        if not line:
            continue
        obj = json.loads(line)
        if obj.get("type") == "event":
            on_event(obj)
        elif obj.get("type") == "summary":
            summary = obj

    stderr = proc.stderr.read() if proc.stderr else ""
    returncode = proc.wait()

    if summary is None:
        raise RuntimeError(f"No Shredator summary received. stderr={stderr!r}")

    if summary.get("exit_code") != returncode:
        raise RuntimeError(f"Exit code mismatch: process={returncode}, json={summary}")

    if not summary.get("success"):
        raise RuntimeError(f"Shredator failed: {summary}; stderr={stderr!r}")

    return summary
```

## Node.js wrapper

```javascript
import { spawn } from "node:child_process";

export function shredatorJson(binary, target) {
  return new Promise((resolve, reject) => {
    const child = spawn(binary, [target, "--force", "--json"], {
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    child.stdout.setEncoding("utf8");
    child.stderr.setEncoding("utf8");

    child.stdout.on("data", chunk => { stdout += chunk; });
    child.stderr.on("data", chunk => { stderr += chunk; });

    child.on("error", reject);
    child.on("close", code => {
      let payload;
      try {
        payload = JSON.parse(stdout);
      } catch (error) {
        reject(new Error(`Invalid Shredator JSON: ${error.message}\nstdout=${stdout}\nstderr=${stderr}`));
        return;
      }

      if (payload.exit_code !== code) {
        reject(new Error(`Exit code mismatch: process=${code}, json=${payload.exit_code}`));
        return;
      }

      if (!payload.success) {
        reject(new Error(`Shredator failed: ${JSON.stringify(payload)}`));
        return;
      }

      resolve(payload);
    });
  });
}
```

## PowerShell wrapper

```powershell
function Invoke-Shredator {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Binary,

        [Parameter(Mandatory = $true)]
        [string]$Target
    )

    $output = & $Binary $Target --force --json 2>&1
    $code = $LASTEXITCODE

    try {
        $payload = $output | ConvertFrom-Json
    }
    catch {
        throw "Shredator did not emit valid JSON. ExitCode=$code Output=$output"
    }

    if ($payload.exit_code -ne $code) {
        throw "Exit code mismatch. Process=$code JSON=$($payload.exit_code)"
    }

    if (-not $payload.success) {
        throw "Shredator failed: $($payload | ConvertTo-Json -Depth 10)"
    }

    return $payload
}
```

## Rust wrapper

```rust
use std::process::Command;

fn shredator_json(binary: &str, target: &str) -> anyhow::Result<serde_json::Value> {
    let output = Command::new(binary)
        .arg(target)
        .arg("--force")
        .arg("--json")
        .output()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let payload: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|err| anyhow::anyhow!("invalid JSON: {err}; stdout={stdout:?}; stderr={stderr:?}"))?;

    let json_exit = payload["exit_code"].as_i64().unwrap_or(-999);
    let process_exit = output.status.code().unwrap_or(-998) as i64;

    if json_exit != process_exit {
        anyhow::bail!("exit code mismatch: process={process_exit}, json={json_exit}");
    }

    if payload["success"] != true {
        anyhow::bail!("shredator failed: {payload}");
    }

    Ok(payload)
}
```

## GUI integration

For a GUI:

1. Ask the user for explicit confirmation in the GUI.
2. Spawn Shredator with `--force --jsonl`.
3. Render progress from events:
   - `file_start`
   - `overwrite_pass_start`
   - `overwrite_pass_complete`
   - `file_removed`
   - `directory_removed`
4. Store the final summary.
5. Show warnings/errors prominently.

## Service integration

For backend services:

- Put Shredator behind a job queue.
- Run with a restricted working directory when possible.
- Use absolute paths.
- Apply your own allowlist/sandbox before invoking Shredator.
- Never expose arbitrary user-provided paths directly to Shredator.
- Record the final summary object.
- Do not log sensitive filenames unless required.

## Avoid shell injection

Bad:

```python
subprocess.run(f"shredator {target} --force --json", shell=True)
```

Good:

```python
subprocess.run(["shredator", str(target), "--force", "--json"])
```

## Wrapper confidence levels

### Minimal success

```text
process return code == 0
```

### Normal success

```text
return code == 0
payload.success == true
payload.status == "completed"
payload.summary.errors == 0
```

### Strict success

```text
return code == 0
payload.success == true
payload.status == "completed"
payload.summary.errors == 0
payload.summary.warnings == 0
```

Strict success is recommended for `colemen_py` if the wrapper needs to be confident before moving on.

## Timeout policy

Wrappers should use a timeout that scales with file size and pass count. A fixed short timeout is a bad idea for large files.

Suggested approach:

```text
estimated_bytes_to_write = file_size * passes
minimum_timeout = 30 seconds
additional_timeout = estimated_bytes_to_write / conservative_bytes_per_second
```

For directories, estimate total bytes before invoking Shredator when practical.

## Cancellation

The current CLI does not have a graceful cancellation protocol. Killing the process may leave a partially overwritten file or partially processed directory.

Wrapper recommendation:

- Avoid killing the process unless necessary.
- If killed, treat the operation as unknown/partial.
- Re-scan the target path.
- Report to the user that secure deletion could not be confirmed.
