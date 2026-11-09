// Run from the repository root:
// npx vitest run --config tools/golden/vitest.themes.config.ts
// Expected values come only from shipped TS functions / the installed defu.
import { mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import { createHash } from 'node:crypto';
import ts from 'typescript';
import { defu } from 'defu';
import { expect, test } from 'vitest';
import { validateTheme, validateThemeVariantConfig } from '../../src/main/themes/themeValidation';
import {
  createVariantTheme, getAssetType, getMimeType, isAudioAssetExtension,
  getSupportedThemeAssetExtensions
} from '../../src/main/themes/themeHelpers';
import type { Theme } from '../../src/shared/types';

type Case = { name: string; input: unknown; expected: unknown };
const output = path.resolve('crates/echosight-core/tests/golden/themes');
const minimal = { id: 'base', name: 'Base' };
const clone = <T>(value: T): T => structuredClone(value);
function save(name: string, cases: Case[]): void {
  writeFileSync(path.join(output, `${name}.json`), JSON.stringify(cases, null, 2) + '\n');
}

// This private pure function is not exported. Transpile its exact source tail,
// not a rewritten oracle, and evaluate it without importing the I/O library.
const librarySource = readFileSync('src/main/themes/themeLibrary.ts', 'utf8');
const marker = 'function compareThemeVersion(';
if (librarySource.indexOf(marker) < 0) throw new Error('version comparator source moved');
const comparatorSource = librarySource.slice(librarySource.indexOf(marker));
const comparatorJs = ts.transpileModule(comparatorSource, {
  compilerOptions: { target: ts.ScriptTarget.ES2022 }
}).outputText;
const compareVersion = vm.runInNewContext(`${comparatorJs}\ncompareThemeVersion`) as
  (left: unknown, right: unknown) => number;

test('generate v1 validation and helper goldens from real TypeScript', () => {
  mkdirSync(output, { recursive: true });
  const validation: Case[] = [];
  const add = (name: string, input: unknown) => {
    validation.push({ name, input, expected: validateTheme(input) });
  };
  for (const input of [null, false, 0, '', 'theme', [], [minimal], {}, minimal]) {
    add(`root ${JSON.stringify(input)}`, input);
  }
  for (const key of ['id', 'name']) {
    for (const value of [null, false, 4, {}, [], '', ' \t\n', '\uFEFF', '\u0085', '\u2003', ' x ']) {
      add(`${key} ${JSON.stringify(value)}`, { ...minimal, [key]: value });
    }
  }
  for (const key of ['author', 'description', 'version', 'isDefault', 'supportsTransparency',
    'fonts', 'effects', 'transparency', 'styles', 'components', 'backgrounds', 'layout',
    'animations', 'compatibility', 'metadata', 'colors', 'customCSS', 'sounds', 'variants']) {
    for (const value of [null, false, 1, '', [], {}, { x: null }]) {
      add(`${key} ${JSON.stringify(value)}`, { ...minimal, [key]: value });
    }
  }
  const paths = [null, false, '', ' ', 'theme.css', 'styles/THEME.CSS', '../theme.css',
    'a/../theme.css', 'a\\..\\theme.css', '/theme.css', '\\theme.css',
    'C:\\theme.css', 'C:/theme.css', 'C:theme.css', '\\\\server\\theme.css',
    'theme\u0000.css', 'theme.txt', '.css', '..css', 'theme.css/', 'theme.css ',
    './theme.css', 'dir/./theme.css'];
  for (const cssFile of paths) add(`css path ${JSON.stringify(cssFile)}`, { ...minimal, cssFile });
  add('recursive CSS and integer key ordering', { ...minimal,
    colors: { z: true, 10: false, 2: [], nested: { bad: null, fine: 0 } },
    customCSS: { raw: 'body {}', '.x': { color: '#fff', opacity: 0.5, nested: { bad: [] } }, bad: 4 }
  });
  add('all errors retain validation order', {
    id: '', name: 42, author: false, description: [], version: null,
    isDefault: 1, supportsTransparency: 'yes', cssFile: '../outside.css',
    customCSS: [], sounds: { enabled: 'yes', volume: 2, events: {
      unknown: null, taskCompleted: '../complete.ogg', backgroundActivated: 'audio/alert.wav',
      undo: { file: 'undo.txt', volume: -1 }, redo: []
    } }, colors: { primary: ['#fff'] }, fonts: null, styles: 'bad', variants: { broken: null }
  });
  for (const volume of [null, '0.5', false, -0.01, 0, 0.5, 1, 1.01]) {
    add(`sound volumes ${JSON.stringify(volume)}`, { ...minimal,
      sounds: { volume, events: { undo: { file: 'undo.wav', volume } } }
    });
  }
  for (const events of [null, [], false, 1, '', {}, { unknown: null, another: [] }]) {
    add(`sound events ${JSON.stringify(events)}`, { ...minimal, sounds: { events } });
  }
  for (const definition of [null, false, 2, [], {}, { file: null }, { file: 2 },
    ...paths, 'audio/ok.wav', 'audio\\ok.wav', 'ok.MP3', 'ok.ogg', 'ok.wav', 'ok.m4a',
    'ok.flac', '.wav', 'ok.wav/', 'C:ok.wav', { file: 'ok.wav', future: null }]) {
    add(`sound definition ${JSON.stringify(definition)}`, { ...minimal,
      sounds: { events: { taskCompleted: definition } }
    });
  }
  add('unknown fields and warning-only values survive verbatim', {
    ...minimal, future: { deep: [null, false, 3] }, assets: 12, preview: null,
    variants: { invalid: null, valid: { name: 3 } }, sounds: { future: [], events: { unknown: null } }
  });
  save('validation', validation);

  const variants: Case[] = [];
  const variantInputs: unknown[] = [null, [], false, 1, '', {}, { name: '', description: '' },
    { name: 123, colors: { primary: false }, customCSS: [] },
    { sounds: { enabled: 'yes', volume: 2, events: { unknown: [], redo: {} } } },
    { cssFile: '../bad.css', supportsTransparency: 42, metadata: [], transparency: null, id: false }];
  for (const key of ['name', 'description', 'customCSS', 'sounds', 'colors', 'effects', 'fonts',
    'styles', 'components', 'backgrounds', 'layout', 'animations']) {
    for (const value of [null, [], false, 0, '', {}, { nested: { bad: null } }]) {
      variantInputs.push({ [key]: value });
    }
  }
  variantInputs.forEach((config, i) => variants.push({ name: `variant ${i}`,
    input: { themeId: 'base', variantId: 'ember.x', config },
    expected: validateThemeVariantConfig('base', 'ember.x', config)
  }));
  save('variants', variants);

  const base: Theme = { ...minimal, description: 'Base theme', path: 'C:\\themes\\base.json',
    colors: { background: { primary: '#000', secondary: '#111' }, text: { primary: '#fff' } },
    components: { button: { border: '1px solid #fff', colors: { text: '#fff' } } },
    variants: { ember: {} }, future: { keep: [1, null] }
  };
  const merges: Case[] = [];
  const addVariant = (name: string, theme: Theme, config: Record<string, unknown>) => {
    const before = clone(theme);
    const expected = createVariantTheme(theme, 'ember', config);
    expect(theme).toEqual(before);
    merges.push({ name, input: { base: theme, variantId: 'ember', config }, expected });
  };
  addVariant('nested merge without base mutation', base, {
    name: 'Ember', colors: { background: { primary: '#300' } },
    components: { button: { colors: { text: '#f90' } } }, ignored: 'not applied', id: 'not-applied'
  });
  for (const value of [null, false, 0, '', ' ', 'css string', [], {}, { x: null }]) {
    addVariant(`minimal override ${JSON.stringify(value)}`, minimal, Object.fromEntries(
      ['name', 'description', 'colors', 'effects', 'fonts', 'styles', 'components',
        'backgrounds', 'layout', 'animations', 'customCSS', 'sounds'].map(key => [key, value])
    ));
  }
  addVariant('empty base description uses name', { ...minimal, description: '' }, {});
  addVariant('arrays concatenate override then default, null falls back', {
    ...minimal, effects: { list: [1, 2], nested: { keep: true, nullBase: null }, zero: 8, blank: 'x' },
    customCSS: 'base CSS', sounds: { volume: 0.8, events: { undo: 'old.wav' } }
  }, {
    effects: { list: [3], nested: { keep: null, nullBase: null }, zero: 0, blank: '' },
    customCSS: '.override {}', sounds: { volume: null, events: { undo: { file: 'new.wav' } } }
  });
  save('merges', merges);

  // Confirm actual dependency behavior rather than trusting the term "deep merge".
  const defuCases: Case[] = [];
  const objects = [{}, { a: null, b: false, c: 0, d: '' },
    { a: [1], b: { keep: true, nested: { x: null } } },
    { a: [2, 3], b: { keep: null, nested: { y: 4 } } },
    { a: { nested: [1] }, b: 'scalar' },
    JSON.parse('{"__proto__":{"x":1},"constructor":{"x":2},"prototype":3,"safe":null}')];
  for (const overrides of objects) for (const defaults of objects) {
    defuCases.push({ name: `defu ${defuCases.length}`, input: { overrides, defaults },
      expected: defu(overrides, defaults) });
  }
  save('defu', defuCases);

  const helpers: Case[] = [];
  const extensions = [...getSupportedThemeAssetExtensions(), '.WEBP', '.OGG', '.bin', '', 'wav'];
  for (const extension of extensions) for (const assetName of [
    'boss-background.webp', 'button-bg', 'button-progress-icon', 'progress-icon',
    'icon-texture', 'texture-border', 'border', 'unknown.bin', 'BG', 'complete'
  ]) helpers.push({ name: `${assetName} ${extension}`, input: { assetName, extension }, expected: {
    assetType: getAssetType(assetName, extension), mimeType: getMimeType(extension),
    audio: isAudioAssetExtension(extension)
  } });
  save('helpers', helpers);
  save('extensions', [{ name: 'supported extensions in source order', input: null,
    expected: getSupportedThemeAssetExtensions() }]);

  const versions: Case[] = [];
  const versionValues = ['1', '1.0.0', '1.2.3', '1.12', '2.0', '  +2x.03beta',
    '1e2.3', '0x10', '-1.2', '', 'v1.0', '1..0', '1.', null, 2, '9007199254740993',
    '9'.repeat(310), '\uFEFF2.0', '\u00852.0'];
  for (const left of versionValues) for (const right of versionValues) {
    versions.push({ name: `version ${versions.length}`, input: { left, right },
      expected: compareVersion(left, right) });
  }
  save('versions', versions);

  const bundled: Case[] = [];
  for (const entry of readdirSync('data/themes', { withFileTypes: true }).sort((a, b) =>
    a.name < b.name ? -1 : a.name > b.name ? 1 : 0)) {
    const file = entry.isDirectory() ? `data/themes/${entry.name}/theme.json` : `data/themes/${entry.name}`;
    if (!file.endsWith('.json')) continue;
    const theme = JSON.parse(readFileSync(file, 'utf8'));
    const expected = validateTheme(theme);
    expect(expected.errors, file).toEqual([]);
    bundled.push({ name: file, input: theme, expected });
  }
  expect(bundled).toHaveLength(11);
  expect(bundled.some(c => c.name === 'data/themes/exiles_corruption.json')).toBe(true);
  save('bundled', bundled);

  const sources = ['src/main/themes/themeValidation.ts', 'src/main/themes/themeHelpers.ts',
    'src/main/themes/themeLibrary.ts', 'node_modules/defu/dist/defu.mjs'];
  writeFileSync(path.join(output, 'provenance.json'), JSON.stringify({
    schema: 1, runtimePlatform: process.platform, node: process.version,
    sources: Object.fromEntries(sources.map(file => [file,
      createHash('sha256').update(readFileSync(file)).digest('hex')])),
    counts: { validation: validation.length, variants: variants.length, merges: merges.length,
      defu: defuCases.length, helpers: helpers.length, extensions: 1,
      versions: versions.length, bundled: bundled.length }
  }, null, 2) + '\n');
});
