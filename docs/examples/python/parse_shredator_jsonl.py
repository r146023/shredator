#!/usr/bin/env python3
"""Run Shredator in JSONL mode and print progress events."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("target", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("shredator"))
    args = parser.parse_args()

    proc = subprocess.Popen(
        [str(args.binary), str(args.target), "--force", "--jsonl"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )

    assert proc.stdout is not None
    summary = None

    for raw_line in proc.stdout:
        obj = json.loads(raw_line)
        if obj.get("type") == "event":
            print(f"[{obj.get('level')}] {obj.get('event')}: {obj.get('message')}")
        elif obj.get("type") == "summary":
            summary = obj

    stderr = proc.stderr.read() if proc.stderr else ""
    returncode = proc.wait()

    if summary is None:
        print(f"No summary received. returncode={returncode} stderr={stderr!r}")
        return 10

    if summary.get("exit_code") != returncode:
        print(f"Exit code mismatch: process={returncode} json={summary.get('exit_code')}")
        return 11

    print("Final summary:")
    print(json.dumps(summary, indent=2))
    return 0 if summary.get("success") else 1


if __name__ == "__main__":
    raise SystemExit(main())
