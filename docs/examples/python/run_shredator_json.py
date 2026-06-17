#!/usr/bin/env python3
"""Run Shredator in JSON mode and enforce strict success."""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("target", type=Path)
    parser.add_argument("--binary", type=Path, default=Path("shredator"))
    parser.add_argument("--passes", type=int, default=3)
    parser.add_argument("--pattern", default="random")
    args = parser.parse_args()

    cmd = [
        str(args.binary),
        str(args.target),
        "--force",
        "--passes",
        str(args.passes),
        "--pattern",
        args.pattern,
        "--json",
    ]

    proc = subprocess.run(cmd, text=True, capture_output=True)

    try:
        payload = json.loads(proc.stdout)
    except json.JSONDecodeError as exc:
        print("Invalid Shredator JSON")
        print(f"returncode={proc.returncode}")
        print(f"stdout={proc.stdout!r}")
        print(f"stderr={proc.stderr!r}")
        raise SystemExit(10) from exc

    if payload.get("exit_code") != proc.returncode:
        print("Exit-code mismatch")
        print(json.dumps(payload, indent=2))
        return 11

    summary = payload.get("summary", {})
    clean = (
        proc.returncode == 0
        and payload.get("success") is True
        and payload.get("status") == "completed"
        and summary.get("errors") == 0
        and summary.get("warnings") == 0
    )

    print(json.dumps(payload, indent=2))
    return 0 if clean else 1


if __name__ == "__main__":
    raise SystemExit(main())
