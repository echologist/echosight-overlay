"""Read-only extraction of public parser evidence; never uses token-mapping.csv.

Usage: python generate.py EVIDENCE_ROOT [--check]
EVIDENCE_ROOT is the original V-ACC research directory. This tool reads its
pinned sources and actual Electron observations, never reruns/writes that oracle.
The committed fixture is portable; paths from the research manifest are omitted.
"""
import argparse
import ast
import hashlib
import json
from pathlib import Path
import re


def rust_string(value):
    return '"' + ''.join(
        {'"': '\\"', '\\': '\\\\', '\n': '\\n', '\r': '\\r', '\t': '\\t'}.get(
            c, c if 32 <= ord(c) < 127 else f'\\u{{{ord(c):x}}}'
        ) for c in value
    ) + '"'


def extract(root):
    selected = [
        'electron/shell/common/keyboard_util.cc',
        'electron/shell/browser/ui/accelerator_util.cc',
        'electron/shell/browser/ui/accelerator_util_unittests.cc',
        'electron/spec/lib/accelerator-helpers.ts',
        'electron/patches/chromium/command-ismediakey.patch',
        'chromium/base/strings/string_util_constants.cc',
        'chromium/ui/events/keycodes/keyboard_codes_win.h',
        'chromium/ui/base/accelerators/global_accelerator_listener/global_accelerator_listener_win.cc',
        'upstream-global-hotkey/src/hotkey.rs',
        'upstream-global-hotkey/src/platform_impl/windows/mod.rs',
    ]
    manifest = json.loads((root / 'source-manifest.json').read_text())
    public_sources = []
    for path in selected:
        entry = next(e for e in manifest if e.get('path') == path and 'sha256' in e)
        assert hashlib.sha256((root / path).read_bytes()).hexdigest() == entry['sha256'], path
        public_sources.append({k: entry[k] for k in ('url', 'sha256')})
    main_entry = next(e for e in manifest if e.get('path') == 'local/hotkeys.ts')
    assert hashlib.sha256((root / 'local/hotkeys.ts').read_bytes()).hexdigest() == main_entry['sha256']
    public_sources.append({
        'repository_path': 'src/main/utils/hotkeys.ts',
        'sha256': main_entry['sha256'],
    })
    source = (root / selected[0]).read_text()
    windows = (root / selected[-1]).read_text()
    # Independent join: Electron VKEY identity -> actual backend VK identity.
    # No table copied from the production Rust adapter or the defective CSV.
    vk_to_code = {}
    for code, vk in re.findall(r'Code::(\w+) => VK_(\w+),', windows):
        # First canonical entry wins (Enter before NumpadEnter, Backslash
        # before IntlYen); never substitute physical aliases sharing a VK.
        vk_to_code.setdefault(vk, code)
    named = re.findall(r'\{"([a-z0-9]+)", \{(ui::VKEY_\w+|CommandOrControl), (\{\}|\'.*?\')\}\}', source)
    chars = re.findall(r"case ('(?:\\.|[^'])+'|0x[0-9A-Fa-f]+):\s*return \{ui::(VKEY_\w+), (\{\}|'.*?')\};", source)
    assert len(named) == 80 and len(chars) == 74, (len(named), len(chars))
    tokens = [(text, vk.removeprefix('ui::'), shifted != '{}') for text, vk, shifted in named]
    tokens += [(chr(int(text, 16)) if text.startswith('0x') else ast.literal_eval(text), vk, shifted != '{}') for text, vk, shifted in chars]
    modifiers = {'VKEY_MENU', 'VKEY_ALTGR', 'VKEY_COMMAND', 'CommandOrControl', 'VKEY_CONTROL', 'VKEY_SHIFT'}
    rows = []
    for text, vk, shift in tokens:
        code = 'None' if vk in modifiers else f'Some(Code::{vk_to_code[vk.removeprefix("VKEY_")]})'
        rows.append(f'    ({rust_string(text)}, {rust_string(vk)}, {code}, {str(shift).lower()}),')
    table = '// Source-derived, independently joined against the pinned Windows backend.\n'
    table += 'pub const SOURCE_TOKENS: &[(&str, &str, Option<Code>, bool)] = &[\n' + '\n'.join(rows) + '\n];\n'
    evidence = json.loads((root / 'electron-results.json').read_text())
    assert evidence['versions']['electron'] == '41.3.0'
    assert evidence['versions']['chrome'] == '146.0.7680.188'
    assert evidence['platform'] == 'linux'
    assert len(evidence['rows']) == 406
    assert len(evidence['upstreamCpp']) == 9
    assert len(evidence['upstreamHelper']) == 2709
    table += 'pub const PAIRS: &[(&str, &str, bool, bool)] = &[\n'
    for row in evidence['rows']:
        table += f'    ({rust_string(row["raw"])}, {rust_string(row["normalized"])}, {str(row["rawElectron"]["accepted"]).lower()}, {str(row["normalizedElectron"]["accepted"]).lower()}),\n'
    table += '];\npub const CPP_CASES: &[(&str, bool)] = &[\n'
    for row in evidence['upstreamCpp']:
        assert row['expected'] == row['actual']
        table += f'    ({rust_string(row["text"])}, {str(row["actual"]).lower()}),\n'
    table += '];\npub const HELPER_ACCEPTED: &[&str] = &[\n'
    for row in evidence['upstreamHelper']:
        assert row['accepted'] and row['isRegistered'] is False
        table += f'    {rust_string(row["text"] )},\n'
    table += '];\n'
    meta = {
        'electron': '41.3.0', 'chromium': '146.0.7680.188',
        'execution_platform': 'linux',
        'evidence_sha256': hashlib.sha256((root / 'electron-results.json').read_bytes()).hexdigest(),
        'evidence_kind': 'Actual globalShortcut.isRegistered conversion, no native registration; chord values separately source-derived.',
        'known_defect': 'Original token-mapping.csv named implied_shift is all true. Not used. Only named plus implies shift.',
        'sources': public_sources,
    }
    return table, json.dumps(meta, indent=2) + '\n'


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('evidence_root', type=Path)
    parser.add_argument('--check', action='store_true')
    args = parser.parse_args()
    table, manifest = extract(args.evidence_root)
    here = Path(__file__).resolve().parent
    for name, content in [('observations.rs', table), ('provenance.json', manifest)]:
        path = here / name
        if args.check:
            assert path.read_text() == content, path
        else:
            path.write_text(content)
    print('Verified 154 source rows, 406 raw/main pairs, 9 C++ cases, 2709 helper acceptances.')
