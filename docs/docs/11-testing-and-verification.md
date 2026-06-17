# Testing and Verification

Testing a destructive tool requires discipline. Never test Shredator on files you care about.

## Create a disposable test directory

```bash
mkdir -p /tmp/shredator-test/nested
printf 'secret alpha\n' > /tmp/shredator-test/a.txt
printf 'secret beta\n' > /tmp/shredator-test/b.tmp
printf 'secret gamma\n' > /tmp/shredator-test/nested/c.txt
```

On Windows PowerShell:

```powershell
New-Item -ItemType Directory -Force .\shredator-test\nested | Out-Null
Set-Content .\shredator-test\a.txt "secret alpha"
Set-Content .\shredator-test\b.tmp "secret beta"
Set-Content .\shredator-test\nested\c.txt "secret gamma"
```

## Test single-file JSON output

```bash
printf 'secret\n' > /tmp/shredator-test/single.txt
shredator /tmp/shredator-test/single.txt --force --json | jq .
test ! -e /tmp/shredator-test/single.txt && echo "file removed"
```

## Test JSONL output

```bash
printf 'secret\n' > /tmp/shredator-test/single-jsonl.txt
shredator /tmp/shredator-test/single-jsonl.txt --force --jsonl | jq .
```

## Test include filtering

```bash
mkdir -p /tmp/shredator-test/filter
printf 'remove me\n' > /tmp/shredator-test/filter/a.tmp
printf 'keep me\n' > /tmp/shredator-test/filter/b.txt

shredator /tmp/shredator-test/filter --force --include "*.tmp" --jsonl | jq .

# b.txt should remain, and directory removal may fail because b.txt remains.
find /tmp/shredator-test/filter -maxdepth 2 -type f -print
```

## Test file-list mode

```bash
mkdir -p /tmp/shredator-test/list
printf 'one\n' > /tmp/shredator-test/list/one.txt
printf 'two\n' > /tmp/shredator-test/list/two.txt
cat > /tmp/shredator-test/targets.txt <<'EOF'
# file list test
/tmp/shredator-test/list/one.txt
/tmp/shredator-test/list/two.txt
/tmp/shredator-test/list/missing.txt
EOF

shredator --file-list /tmp/shredator-test/targets.txt --force --jsonl | jq .
```

Expected:

- Two successful file removals.
- One skipped missing path warning.
- Final status may still be completed if there are skipped paths but no failed processed paths.

## Test cancelled machine mode

```bash
printf 'important\n' > /tmp/shredator-test/important.pdf
shredator /tmp/shredator-test/important.pdf --json | jq .
```

Expected:

- `status: "cancelled"`
- `exit_code: 3`
- `success: false`
- `confirmation_required` event
- File should still exist.

## Test forced machine mode

```bash
shredator /tmp/shredator-test/important.pdf --force --json | jq .
```

Expected:

- `status: "completed"`
- `exit_code: 0`
- File removed.

## Test benchmark

```bash
dd if=/dev/zero of=/tmp/shredator-test/bench.bin bs=1M count=64
shredator /tmp/shredator-test/bench.bin --force --passes 1 --pattern zeros --benchmark --json | jq .
```

PowerShell equivalent:

```powershell
$bytes = New-Object byte[] (64MB)
[IO.File]::WriteAllBytes(".\bench.bin", $bytes)
& shredator.exe .\bench.bin --force --passes 1 --pattern zeros --benchmark --json | ConvertFrom-Json
```

## What verification can and cannot prove

You can verify:

- The target path no longer exists.
- Shredator reported completed status.
- The process exit code matched the JSON exit code.
- The summary counters look correct.
- The event stream contains completed overwrite passes.

You cannot easily prove:

- No physical remnants exist on an SSD.
- No filesystem journal or snapshot contains old data.
- No backup contains a copy.
- No application cache contains a copy.

## Suggested automated tests for wrappers

A wrapper test suite should cover:

1. Missing binary.
2. Missing target path.
3. Successful single-file delete.
4. Successful empty-file delete.
5. JSON parse failure handling.
6. JSON exit-code mismatch handling.
7. Cancelled machine mode without `--force`.
8. File-list with mixed success/skips.
9. Warning handling policy.
10. Timeout/kill behavior.

## Golden output warning

Do not make brittle tests that compare full JSON output exactly. Fields like `duration_ms`, messages, paths, and event ordering can vary.

Prefer checking stable fields:

```text
schema
success
status
exit_code
summary.files_shredded
summary.errors
summary.warnings
event names
```
