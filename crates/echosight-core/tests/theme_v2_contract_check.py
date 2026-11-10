"""Independent draft-2020-12 contract and source-integrity check.
Run: python crates/echosight-core/tests/theme_v2_contract_check.py
Requires the development-only Python jsonschema package, not a runtime/Cargo dependency.
Semantic formats/references/cycles are covered by Rust; this checks the structural contract.
"""
import hashlib
import json
from pathlib import Path

from jsonschema import Draft202012Validator

HERE = Path(__file__).resolve().parent
CORE = HERE.parent
REPO = CORE.parent.parent
schema = json.loads((CORE / "schema/theme-v2.schema.json").read_text())
Draft202012Validator.check_schema(schema)
validator = Draft202012Validator(schema)
cases = json.loads((HERE / "theme_v2_cases.json").read_text())
for case in cases:
    errors = list(validator.iter_errors(case["document"]))
    assert (not errors) == case["valid"], (case["name"], [e.message for e in errors])
manifest = json.loads((HERE / "theme_v2_fixtures/manifest.json").read_text())
for item in manifest["files"]:
    data = (REPO / "data/themes" / item["path"]).read_bytes()
    assert hashlib.sha256(data).hexdigest() == item["sha256"], item["path"]
for theme in manifest["themes"]:
    converted = json.loads((HERE / "theme_v2_fixtures" / theme["id"] / "converted.json").read_text())
    validator.validate(converted["theme"])
print(f"{len(cases)} schema/runtime corpus cases; {len(manifest['themes'])} converted structures; "
      f"{len(manifest['files'])} original source hashes verified")
