# Recipes

These are copy/paste-oriented examples for common Shredator workflows.

## 1. Shred one file

```bash
shredator ./secret.txt
```

Use this for manual use when you are comfortable answering confirmation prompts if they appear.

## 2. Shred one file without prompting

```bash
shredator ./secret.txt --force
```

Use this when your script or wrapper has already confirmed user intent.

## 3. Shred one file and get JSON

```bash
shredator ./secret.txt --force --json
```

Good for Python/Rust/Node wrappers that want one final structured result.

## 4. Shred one file with streaming progress

```bash
shredator ./secret.txt --force --jsonl
```

Good for a GUI progress panel or long-running integration.

## 5. Fast one-pass zero overwrite

```bash
shredator ./secret.txt --force --passes 1 --pattern zeros
```

Use when speed matters and you want simple deletion hygiene.

## 6. Three-pass random overwrite

```bash
shredator ./secret.txt --force --passes 3 --pattern random
```

This is effectively the default behavior plus explicitness.

## 7. DoD-style three-pass overwrite

```bash
shredator ./secret.txt --force --passes 3 --pattern dod
```

This writes zeros, ones, and random data in a repeating cycle.

## 8. Gutmann-style overwrite

```bash
shredator ./secret.txt --force --pattern gutmann
```

This forces 35 passes. Expect it to be much slower.

## 9. Rename before delete

```bash
shredator ./secret-client-list.xlsx --force --zero-names
```

This renames the file to a random hexadecimal filename before removing it.

## 10. Benchmark a file

```bash
shredator ./large.bin --force --passes 1 --pattern zeros --benchmark
```

Use a disposable file, not a valuable file.

## 11. Benchmark random vs zero pattern

```bash
cp ./test.bin ./test-random.bin
cp ./test.bin ./test-zeros.bin

shredator ./test-random.bin --force --passes 1 --pattern random --benchmark
shredator ./test-zeros.bin  --force --passes 1 --pattern zeros  --benchmark
```

This can help you understand the overhead of random generation.

## 12. Shred a directory recursively

```bash
shredator ./old-export --force
```

Every file in the directory tree is processed, then directories are removed.

## 13. Shred only `.tmp` files in a directory

```bash
shredator ./cache --force --include "*.tmp"
```

Directories are still traversed, but files that do not match are skipped. Directory removal may fail if skipped files remain.

## 14. Shred everything except images

```bash
shredator ./export --force --exclude "*.jpg" --exclude "*.png"
```

Excluded files remain. Directory removal may fail if excluded files remain inside directories.

## 15. Shred files with names containing `secret`

```bash
shredator ./data --force --include "*secret*"
```

This matches final filenames like:

- `secret.txt`
- `client-secret.xlsx`
- `old_secret_backup.csv`

## 16. Process only the top directory level

```bash
shredator ./data --force --max-depth 0
```

This processes files directly in `./data` but does not recurse into nested directories.

## 17. Process one nested level

```bash
shredator ./data --force --max-depth 1
```

This processes files in `./data` and files in immediate child directories.

## 18. Build a file list manually

Create `targets.txt`:

```text
# Paths to shred
./a.txt
./b.txt
./old-cache
```

Run:

```bash
shredator --file-list ./targets.txt --force
```

## 19. Generate a file list with `find`

```bash
find ./cache -type f -name "*.tmp" > targets.txt
shredator --file-list targets.txt --force --json
```

## 20. Generate a file list from PowerShell

```powershell
Get-ChildItem .\cache -Recurse -File -Filter *.tmp |
    ForEach-Object { $_.FullName } |
    Set-Content .\targets.txt

& shredator.exe --file-list .\targets.txt --force --json
```

## 21. Parse JSON with `jq`

```bash
shredator ./secret.txt --force --json | jq .
```

Check success:

```bash
if shredator ./secret.txt --force --json | jq -e '.success == true and .summary.errors == 0'; then
  echo "ok"
else
  echo "failed"
fi
```

## 22. Watch JSONL progress with `jq`

```bash
shredator ./big-dir --force --jsonl | jq -r '
  if .type == "event" then
    "[\(.level)] \(.event): \(.message)"
  else
    "SUMMARY status=\(.status) files=\(.summary.files_shredded) errors=\(.summary.errors)"
  end
'
```

## 23. Strict wrapper run in Bash

```bash
set -euo pipefail

out="$(shredator ./secret.txt --force --json)"
status="$(printf '%s' "$out" | jq -r '.status')"
warnings="$(printf '%s' "$out" | jq -r '.summary.warnings')"
errors="$(printf '%s' "$out" | jq -r '.summary.errors')"

if [ "$status" != "completed" ] || [ "$warnings" != "0" ] || [ "$errors" != "0" ]; then
  echo "Shredator did not complete cleanly"
  printf '%s\n' "$out" | jq .
  exit 1
fi
```

## 24. Strict wrapper run in Python

```python
import json
import subprocess

cmd = ["shredator", "./secret.txt", "--force", "--json"]
result = subprocess.run(cmd, text=True, capture_output=True)
payload = json.loads(result.stdout)

if result.returncode != 0:
    raise RuntimeError(payload)
if not payload["success"]:
    raise RuntimeError(payload)
if payload["summary"]["warnings"] or payload["summary"]["errors"]:
    raise RuntimeError(f"Shredator completed with warnings/errors: {payload}")
```

## 25. Delete temp files older than a policy window

Shredator does not select files by age itself. Use your shell/tooling to generate a file list.

Linux/macOS:

```bash
find ./tmp -type f -mtime +30 > old-temp-targets.txt
shredator --file-list old-temp-targets.txt --force --jsonl
```

PowerShell:

```powershell
$cutoff = (Get-Date).AddDays(-30)
Get-ChildItem .\tmp -File -Recurse |
    Where-Object { $_.LastWriteTime -lt $cutoff } |
    ForEach-Object { $_.FullName } |
    Set-Content .\old-temp-targets.txt

& shredator.exe --file-list .\old-temp-targets.txt --force --jsonl
```

## 26. Shred an export after successful upload

```bash
if upload-tool ./export.zip; then
  shredator ./export.zip --force --json
else
  echo "Upload failed; not deleting export"
fi
```

## 27. Use with application quarantine folders

```bash
mkdir -p ./quarantine
mv ./unsafe-output/* ./quarantine/
shredator ./quarantine --force --jsonl
```

## 28. Shred matching browser/cache artifacts

```bash
find ./cache -type f \( -name "*.tmp" -o -name "*.cache" -o -name "*.bak" \) > targets.txt
shredator --file-list targets.txt --force --passes 1 --pattern zeros --jsonl
```

## 29. Keep logs separate from machine output

```bash
shredator ./secret.txt --force --json > shredator-result.json 2> shredator-stderr.log
```

Machine-readable structured output is printed to stdout. Fatal errors may also produce stderr text depending on context, so capture both in wrappers.

## 30. Avoid shell history leaks

Instead of typing sensitive paths directly into an interactive shell, put them in a temporary file list generated by your application, then shred the list file too if needed.

```bash
printf '%s\n' "$SENSITIVE_PATH" > targets.txt
shredator --file-list targets.txt --force --json
shredator targets.txt --force --passes 1 --pattern zeros
```

Better: avoid writing sensitive target lists to disk at all in future versions by adding stdin support.
