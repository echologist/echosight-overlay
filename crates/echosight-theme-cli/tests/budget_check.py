#!/usr/bin/env python3
"""Linux executable regression: never run amplification in an unlimited child.

Usage: python3 budget_check.py /absolute/path/to/echosight-theme evidence-dir
Keeps exact fixtures, hashes, stdout/stderr, exit status and per-child peak RSS.
The ordinary Linux Cargo integration suite invokes this too (not ignored).
"""

import hashlib
import json
import os
from pathlib import Path
import resource
import signal
import subprocess
import sys
import time


BUDGET_MESSAGE = "v1 validation exceeds 4096 nodes, 64 levels or 256-byte keys"


def limits():
    address_space = 512 * 1024 * 1024
    resource.setrlimit(resource.RLIMIT_AS, (address_space, address_space))
    resource.setrlimit(resource.RLIMIT_CPU, (15, 15))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def snapshot(root):
    return {
        str(p.relative_to(root)): (
            "directory" if p.is_dir() else hashlib.sha256(p.read_bytes()).hexdigest()
        )
        for p in sorted(root.rglob("*"))
    }


def cases():
    base = {"id": "diagnostic-amplification", "name": "Diagnostic amplification"}
    yield "diagnostic-amplification", {
        **base,
        "colors": {"x" * 32768: {str(i): None for i in range(20000)}},
    }
    yield "long-key", {**base, "colors": {"x" * 257: None}}
    yield "many-nodes", {**base, "extension": [None] * 4096}
    nested = None
    for _ in range(65):
        nested = {"child": nested}
    yield "deep-extension", {**base, "extension": nested}
    # Overlapping opaque base/variant maps would be cloned by legacy defu helpers.
    # Stay below the core's depth/node limits: an overlong key only in the
    # overlapping variant subtree must be rejected before legacy defu clones.
    defaults, overrides = {"payload": "x" * 262144}, {"x" * 257: None}
    for _ in range(30):
        defaults, overrides = {"child": defaults}, {"child": overrides}
    yield "nested-variant", {
        **base,
        "styles": defaults,
        "variants": {"night": {"styles": overrides}},
    }


def main():
    binary = Path(sys.argv[1]).resolve(strict=True)
    evidence = Path(sys.argv[2]).resolve()
    evidence.mkdir(parents=True, exist_ok=True)
    results = []
    for name, document in cases():
        work = evidence / name
        work.mkdir()  # Never reuse evidence or an old conversion output.
        source = work / "source"
        source.mkdir()
        fixture = source / "theme.json"
        data = json.dumps(document, separators=(",", ":")).encode()
        fixture.write_bytes(data)
        before = snapshot(work)
        for command in ("validate", "convert"):
            argv = [str(binary), "--json", command, str(fixture)]
            if command == "convert":
                argv.append(str(work / "output"))
            # Logs live outside work so *all* filesystem changes are detectable.
            stdout = evidence / f"{name}-{command}.stdout"
            stderr = evidence / f"{name}-{command}.stderr"
            with stdout.open("w+") as out, stderr.open("w+") as err:
                child = subprocess.Popen(
                    argv, cwd=work, stdout=out, stderr=err, preexec_fn=limits
                )
                deadline = time.monotonic() + 25
                timed_out = False
                while True:
                    pid, status, usage = os.wait4(child.pid, os.WNOHANG)
                    if pid:
                        break
                    if time.monotonic() >= deadline:
                        timed_out = True
                        os.kill(child.pid, signal.SIGKILL)
                        _, status, usage = os.wait4(child.pid, 0)
                        break
                    time.sleep(0.01)
                child.returncode = os.waitstatus_to_exitcode(status)
                out.seek(0)
                err.seek(0)
                output, error = out.read(), err.read()
            try:
                response = json.loads(output)
            except json.JSONDecodeError:
                response = None
            expected = {
                "ok": False,
                "command": command,
                "diagnostics": [{
                    "severity": "error", "location": "", "code": "budget",
                    "message": BUDGET_MESSAGE, "value": None,
                }],
            }
            unchanged = snapshot(work) == before
            result = {
                "case": name, "argv": argv,
                "input_bytes": len(data),
                "input_sha256": hashlib.sha256(data).hexdigest(),
                "exit": child.returncode, "timed_out": timed_out,
                "max_rss_kib": usage.ru_maxrss, "user_seconds": usage.ru_utime,
                "stdout": output, "stderr": error,
                "tree_unchanged": unchanged,
                "passed": child.returncode == 1 and response == expected
                and error == "" and unchanged and not timed_out,
            }
            results.append(result)
    report = {
        "binary": str(binary),
        "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
        "limits": {"address_space_bytes": 512 * 1024 * 1024,
                   "cpu_seconds": 15, "core_bytes": 0},
        "results": results,
    }
    (evidence / "results.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))
    assert len(results) == 10
    return 0 if all(r["passed"] for r in results) else 1


if __name__ == "__main__":
    sys.exit(main())
