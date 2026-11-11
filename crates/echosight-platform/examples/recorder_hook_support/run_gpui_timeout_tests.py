#!/usr/bin/env python3
"""Run the opt-in S9 headless GPUI probes without editing dependency manifests.

Requires a dedicated CARGO_TARGET_DIR. This builds a Rust test harness in the
example executable slot: it is NOT a runnable native S9 artifact.
"""
import json
import os
from pathlib import Path
import re
import subprocess
import sys


def main():
    if not os.environ.get("CARGO_TARGET_DIR"):
        raise SystemExit("Set a dedicated CARGO_TARGET_DIR for this test harness")
    root = Path(__file__).resolve().parents[4]
    command = [
        "cargo", "rustc", "--locked", "-p", "echosight-platform",
        "--example", "s9_recorder_hook", "--features", "gpui/test-support",
        "--message-format=json", "--", "--test", "--cfg", "s9_gpui_timeout_tests",
        "--check-cfg", "cfg(s9_gpui_timeout_tests)",
    ]
    build = subprocess.run(command, cwd=root, text=True, stdout=subprocess.PIPE)
    messages = [json.loads(line) for line in build.stdout.splitlines()]
    for message in messages:
        if message.get("reason") == "compiler-message":
            print(message["message"].get("rendered", ""), file=sys.stderr)
    if build.returncode:
        raise SystemExit(build.returncode)
    executables = [
        message["executable"] for message in messages
        if message.get("reason") == "compiler-artifact"
        and message.get("target", {}).get("name") == "s9_recorder_hook"
        and message.get("executable")
    ]
    if len(executables) != 1:
        raise SystemExit(f"Expected one exact S9 executable, got {executables!r}")
    executable = executables[0]
    expected = {
        "gpui_timeout_tests::long_only_prefix_has_pending_input_but_no_timeout",
        "gpui_timeout_tests::ambiguous_prefix_has_real_timeout_and_replays_short_action",
        "gpui_timeout_tests::completed_long_chord_cancels_short_timeout",
    }
    listing = subprocess.check_output(
        [executable, "gpui_timeout_tests::", "--list"], text=True, cwd=root
    )
    listed = set(re.findall(r"^(.*): test$", listing, re.MULTILINE))
    if listed != expected:
        raise SystemExit(f"Wrong/missing GPUI probes: {listed!r}")
    run = subprocess.run(
        [executable, "gpui_timeout_tests::", "--nocapture"],
        text=True, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
    )
    print(run.stdout, end="")
    passed = set(re.findall(r"^test (\S+) \.\.\. ok$", run.stdout, re.MULTILINE))
    if run.returncode or passed != expected:
        raise SystemExit(run.returncode or "Expected all named GPUI probes to execute")
    print(f"Verified {len(passed)} named headless GPUI probes; no Windows runtime evidence.")


if __name__ == "__main__":
    main()
