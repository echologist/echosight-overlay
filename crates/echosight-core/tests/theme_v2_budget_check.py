"""Build the CURRENT production API, then probe it in bounded Linux children.

Run from any directory: python crates/echosight-core/tests/theme_v2_budget_check.py
Optional positional JSON paths rerun exact review payloads (expected budget errors).
No process limits are applied to Cargo or the caller. Non-Linux skips only probes.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[3]
ENV = {**os.environ, "CARGO_TARGET_DIR": str(ROOT / "target")}
build = subprocess.run(
    ["cargo", "test", "--locked", "-p", "echosight-core", "--test",
     "theme_v2_budget_probe", "--no-run", "--message-format=json"],
    cwd=ROOT, env=ENV, text=True, capture_output=True, check=True,
)
artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith("{")]
probe = next(a["executable"] for a in artifacts
             if a.get("reason") == "compiler-artifact" and a.get("executable")
             and a["target"]["name"] == "theme_v2_budget_probe")
print(json.dumps({"current_api_test_executable": probe}), flush=True)
if sys.platform != "linux":
    print("SKIP: RLIMIT_AS / wait4 peak RSS probes require Linux; portable Rust tests still apply")
    sys.exit(0)
import resource


def limits():
    resource.setrlimit(resource.RLIMIT_AS, (512 * 1024 * 1024, 512 * 1024 * 1024))
    resource.setrlimit(resource.RLIMIT_CPU, (30, 30))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def doc(future, names):
    slots = {"button": {"fill": "red", "future": future}}
    slots.update({name: {"$extends": "button"} for name in names if name != "button"})
    return {"format": 2, "id": "x", "name": "x", "slots": slots}


schema = json.loads((ROOT / "crates/echosight-core/schema/theme-v2.schema.json").read_text())
names = schema["$defs"]["slots"]["properties"]
cases = [("known-array-90000", doc([0] * 90000, names)),
         ("known-array-50000", doc([0] * 50000, names)),
         ("string-1000", doc("x" * 65536, [f"s{i}" for i in range(1000)])),
         ("string-3000", doc("x" * 65536, [f"s{i}" for i in range(3000)]))]
# These have few warnings/nodes: they must hit allocation bytes/work, not
# accidentally pass only because unknown slot diagnostic count is capped.
cases.append(("known-string-bytes", doc("x" * (1024 * 1024), names)))
reference_doc = doc("small", [])
reference_doc["tokens"] = {"future": {"shape": {"color": "red", "future": "x" * (1024 * 1024)}}}
reference_doc["slots"] = {name: {"fill": "@future.shape"} for name in names}
cases.append(("reference-string-bytes", reference_doc))
reference_nodes = doc("small", [])
reference_nodes["tokens"] = {"future": {"shape": {"color": "red", "future": [0] * 5000}}}
reference_nodes["slots"] = {name: {"fill": "@future.shape"} for name in names}
cases.append(("reference-visits", reference_nodes))
variant_doc = doc("x" * 65536, [])
variant_doc["variants"] = {f"v{i}": {} for i in range(100)}
cases.append(("variant-aggregate", variant_doc))
chain = doc([0] * 1000, [])
previous = "button"
for name in names:
    if name != "button":
        chain["slots"][name] = {"$extends": previous, f"future_{name}": [0] * 1000}
        previous = name
cases.append(("inheritance-chain-work", chain))
failed = False
with tempfile.TemporaryDirectory(prefix="theme-v2-budget-") as directory:
    inputs = []
    for name, value in cases:
        path = Path(directory) / (name + ".json")
        path.write_text(json.dumps(value, separators=(",", ":")))
        inputs.append(path)
    inputs.extend(Path(arg).resolve() for arg in sys.argv[1:])
    for index, path in enumerate(inputs):
        with open(Path(directory) / f"output-{index}", "w+") as output:
            child = subprocess.Popen(
                [probe, "--ignored", "--exact", "budget_probe", "--nocapture"],
                env={**ENV, "THEME_V2_BUDGET_INPUT": str(path)},
                stdout=output, stderr=subprocess.STDOUT, preexec_fn=limits,
            )
            _, status, usage = os.wait4(child.pid, 0)
            child.returncode = os.waitstatus_to_exitcode(status)
            output.seek(0)
            text = output.read()
        result = next((json.loads(line.split("=", 1)[1]) for line in text.splitlines()
                       if line.startswith("BUDGET_RESULT=")), None)
        passed = child.returncode == 0 and result is not None and not result["ok"] and result["budget"]
        if passed and result is not None and path.stem in {"known-string-bytes", "reference-string-bytes",
                                    "variant-aggregate", "inheritance-chain-work"}:
            passed = any("materialization budget" in message for message in result["budget_messages"])
        if passed and result is not None and path.stem == "reference-visits":
            passed = any("100000 visits" in message for message in result["budget_messages"])
        failed |= not passed
        print(json.dumps({"case": path.name, "input_bytes": path.stat().st_size,
                          "exit": child.returncode, "peak_rss_kib": usage.ru_maxrss,
                          "result": result, "pass": passed,
                          **({"output": text} if not passed else {})}), flush=True)
sys.exit(1 if failed else 0)
