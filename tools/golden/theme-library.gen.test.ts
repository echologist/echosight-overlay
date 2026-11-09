import { promises as fs } from 'node:fs';
import path from 'node:path';
import os from 'node:os';
import { createHash } from 'node:crypto';
import { expect, test, vi } from 'vitest';
import { ThemeLibrary } from '../../src/main/themes/themeLibrary';
import { loadThemeAssetData, loadThemeCssFile } from '../../src/main/themes/themeAssetLoaders';
import { createThemeSoundController, resolveThemeSoundDefinition } from '../../src/renderer/features/themes/themeSounds';
import type { Theme, ThemeSoundEvent } from '../../src/shared/types';

const output = 'crates/echosight-core/tests/golden/theme-library/pipeline.json';
const stamp = '2025-01-02T03:04:05.000Z';
const theme = (id: string, extra = {}) => JSON.stringify({ id, name: id, ...extra });
type Files = Record<string, string>;
const fixture = {
  'a.json': theme('duplicate', { marker: 'first', assets: { invented: {} }, disabled: true }),
  'b.json': theme('standalone', { cssFile: 'single.css', variants: { unused: {} } }),
  'single.css': '.single { background: url(./main_bg.svg); }',
  'bad.json': theme('bad', { colors: 'bad' }),
  'disabled.json': theme('disabled', { disabled: true, enabled: false, unknown: 7 }),
  'many-errors.json': '{"name":null,"colors":false,"supportsTransparency":null}',
  'array-variants/theme.json': theme('array-variants', { variants: [{ name: 'array child' }, []] }),
  'path-variants/theme.json': theme('path-variants', { variants: { nested: {}, 'nested/child': {}, '..': {} } }),
  'path-variants/variants/nested/child/a.wav': 'child',
  'abort-variants/theme.json': theme('abort-variants', { variants: { good: {}, '../../../escape': {} } }),
  'broken.json': '{ nope',
  'bom.json': '\ufeff' + theme('bom'),
  'not-object.json': 'null',
  'proto.json': '{"id":"proto","name":"proto","__proto__":{"polluted":true}}',
  'not-json.JSON': theme('ignored'),
  'empty/readme.txt': 'no theme',
  'folder/theme.json': theme('folder', {
    author: 'test', sounds: { enabled: false, events: { undo: 'complete.ogg', future: 'a.wav' } },
    cssFile: 'styles.css', animations: {}, colors: { a: 'base' },
    variants: { '10': { name: 'ten' }, '2': {}, red: { colors: { a: 'red' }, sounds: { enabled: true } }, bad: { colors: 4 }, scalar: 7 }
  }),
  'folder/styles.css': '.a { background: url("./main_bg.svg"); mask: url(\'./sub/a.b.png\'); x: url(../no.png); y: url(https://no); z: url(./😀.svg); }',
  'folder/main_bg.svg': '<svg/>', 'folder/complete.ogg': 'audio',
  'folder/CAPS.PNG': 'caps', 'folder/same.png': 'png', 'folder/same.svg': 'svg',
  'folder/preview.jpg': 'jpg', 'folder/preview.png': 'png',
  'folder/README.md': '# Docs', 'folder/info.txt': 'ignored',
  'folder/ignored.css': 'ignored', 'folder/nested/not-discovered.png': 'ignored',
  'folder/variants/red/main_bg.svg': 'red', 'folder/variants/red/alert.wav': 'alert',
  'z.json': theme('duplicate', { marker: 'last' })
};
const defaults: Files = {}, targets: Files = {};
for (const [name, source, target] of [
  ['newer', { version: '2.0' }, { version: '1.0' }],
  ['equal', { version: '1.0.0' }, { version: '1' }],
  ['older', { version: '1' }, { version: '2' }],
  ['custom-author', { version: '2' }, { version: '1', author: 'user' }],
  ['custom-id', { version: '2' }, { version: '1', id: 'user' }],
  ['invalid-version', { version: 'x' }, { version: '1' }],
  ['no-author', { version: '2', author: undefined }, { version: '1', author: undefined }],
  ['suffix', { version: '2beta' }, { version: '1' }]
] as const) {
  defaults[`${name}.json`] = theme(name, { author: 'bundled', ...source });
  targets[`${name}.json`] = theme(name, { author: 'bundled', ...target, custom: true });
}
defaults['missing.json'] = theme('missing');
defaults['numeric-author.json'] = '{"id":"numeric","name":"numeric","author":1.0,"version":"2"}';
targets['numeric-author.json'] = '{"id":"numeric","name":"numeric","author":1,"version":"1"}';
defaults['object-author.json'] = theme('object-author', { author: {}, version: '2' });
targets['object-author.json'] = theme('object-author', { author: {}, version: '1' });
defaults['null-author.json'] = theme('null-author', { author: null, version: '2' });
targets['null-author.json'] = theme('null-author', { version: '1' });
defaults['broken.json'] = theme('broken', { version: '2' }); targets['broken.json'] = '{ bad';
defaults['folder/theme.json'] = theme('folder', { version: '2' });
defaults['folder/nested/a.svg'] = 'new'; targets['folder/theme.json'] = theme('folder', { version: '1' });
targets['folder/nested/a.svg'] = 'old'; targets['folder/custom.svg'] = 'retain';
defaults['missing-folder/theme.json'] = theme('missing-folder'); defaults['missing-folder/a.wav'] = 'audio';

async function writeFiles(root: string, files: Files) {
  for (const [name, bytes] of Object.entries(files)) {
    const target = path.join(root, name); await fs.mkdir(path.dirname(target), { recursive: true }); await fs.writeFile(target, bytes);
  }
}
async function tree(root: string): Promise<Record<string, string>> {
  const files: Record<string, string> = {};
  for (const entry of (await fs.readdir(root, { withFileTypes: true })).sort((a, b) => a.name < b.name ? -1 : 1)) {
    if (entry.isDirectory()) for (const [key, val] of Object.entries(await tree(path.join(root, entry.name)))) files[`${entry.name}/${key}`] = val;
    else files[entry.name] = (await fs.readFile(path.join(root, entry.name))).toString('base64');
  }
  return files;
}
async function provenance(base = '.') {
  const files = ['tools/golden/theme-library.gen.test.ts', 'tools/golden/vitest.theme-library.config.ts', 'src/main/themes/themeLibrary.ts', 'src/main/themes/themeAssetLoaders.ts', 'src/main/themes/themeHelpers.ts',
    'src/main/themes/themeValidation.ts', 'src/main/utils/pathSafety.ts', 'src/main/utils/jsonStorage.ts', 'src/shared/errors.ts',
    'src/renderer/features/themes/themeSounds.ts', 'node_modules/defu/package.json', 'node_modules/defu/dist/defu.mjs', 'node_modules/defu/dist/defu.cjs', 'node_modules/destr/package.json', 'node_modules/destr/dist/index.mjs', 'node_modules/destr/dist/index.cjs'];
  const result: Record<string, string> = {};
  for (const file of files) result[file] = createHash('sha256').update(await fs.readFile(path.join(base, file))).digest('hex');
  for (const [file, bytes] of Object.entries(await tree(path.join(base, 'data/themes')))) result[`data/themes/${file}`] = createHash('sha256').update(Buffer.from(bytes, 'base64')).digest('hex');
  return result;
}
async function generate() {
  const root = await fs.mkdtemp(path.join(process.env.TMPDIR || os.tmpdir(), 'theme-library-'));
  vi.useFakeTimers({ toFake: ['Date'] }); vi.setSystemTime(new Date(stamp));
  const normalize = (value: unknown): unknown => {
    if (typeof value === 'string') { const result = value.split(root).join('$ROOT'); return result.startsWith('$ROOT') ? result.replace(/\\/g, '/') : result; }
    if (Array.isArray(value)) return value.map(normalize);
    if (value && typeof value === 'object') return Object.fromEntries(Object.entries(value).map(([key, value]) => [key, normalize(value)]));
    return value;
  };
  try {
    const cases = [];
    for (const [name, before, bundled] of [['discovery', fixture, {}], ['sync', targets, defaults], ['bundled', {}, {}]] as const) {
      const dir = path.join(root, name); const themesDir = path.join(dir, 'themes'); const defaultThemesDir = path.join(dir, 'defaults');
      await fs.mkdir(themesDir, { recursive: true }); await fs.mkdir(defaultThemesDir, { recursive: true });
      await writeFiles(themesDir, before); await writeFiles(defaultThemesDir, bundled);
      if (name === 'bundled') await fs.cp('data/themes', defaultThemesDir, { recursive: true });
      const warnings: string[] = []; const errors: string[] = [];
      const library = new ThemeLibrary({ dataDir: dir, themesDir, defaultThemesDir, isPackaged: false, platform: 'linux',
        logger: { log() {}, warn(message) { warnings.push(String(message)); }, error(message) { errors.push(String(message)); } } });
      await library.initialize();
      const first = normalize([...library.themes.values()]);
      const deliveries = [];
      for (const t of library.themes.values()) {
        if (t.cssFile) deliveries.push({ id: t.id, kind: 'css', name: t.cssFile, result: await loadThemeCssFile(t, t.cssFile) });
        if (name === 'discovery') for (const asset of Object.keys(t.assets || {})) deliveries.push({ id: t.id, kind: 'asset', name: asset, result: await loadThemeAssetData(t, asset) });
      }
      if (name === 'discovery') for (const request of ['other.css', '../outside.css', 'C:\\evil.css', '/tmp/no.css', 'styles.css', './styles.css', 'sub\\..\\styles.css', 'styles.css\0']) {
        deliveries.push({ id: 'folder', kind: 'css', name: request, result: await loadThemeCssFile(library.themes.get('folder'), request) });
      }
      const firstWarnings = [...warnings], firstErrors = [...errors]; warnings.length = 0; errors.length = 0;
      await library.reloadThemes();
      expect(normalize([...library.themes.values()])).toEqual(first);
      expect(warnings).toEqual(firstWarnings); expect(errors).toEqual(firstErrors);
      cases.push(normalize({ name, before, bundled, themes: first, warnings, errors, deliveries, after: name === 'sync' ? await tree(themesDir) : null }));
    }
    const sounds = [];
    for (const definition of ['dir\\complete.ogg', 'sub/complete.ogg', 'trailing/', '.wav', '', { file: 'alert.wav', volume: .25 }, { file: 'a', volume: 4 }, { file: 'a', volume: -1 }, { file: 'a', volume: 'bad' }, null]) {
      for (const enabled of [true, false]) {
        const t = { id: 's', name: 's', sounds: { enabled, volume: .8, events: { taskCompleted: definition } } } as unknown as Theme;
        for (const settings of [{ enabled: true, volume: 50 }, { enabled: false, volume: 50 }, { enabled: true, volume: 200 }, { enabled: true, volume: -2 }]) {
          const requests: string[] = [], played: { source: string; volume: number }[] = [];
          const controller = createThemeSoundController({ api: { async getThemeAsset(_id, name) { requests.push(name); return { data: 'YQ==', mimeType: 'audio/ogg', extension: '.ogg', type: 'sound', isText: false }; } }, createAudio(source) { return { currentTime: 0, volume: 0, async play() { played.push({ source, volume: this.volume }); } }; } });
          controller.applyTheme(t, settings); await controller.play('taskCompleted');
          sounds.push({ theme: t, event: 'taskCompleted' as ThemeSoundEvent, settings, definition: resolveThemeSoundDefinition(t, 'taskCompleted'), requests, played });
        }
      }
    }
    const assetFiles: Files = {
      'style.css': 'url(./😀.svg) url(./\u0085.png) url(./\ufeff.png)',
      '.css': 'hidden', 'image.svg': '<svg/>', 'audio.wav': 'audio',
      'audio.mp3': 'mp3', 'audio.m4a': 'm4a', 'blob.bin': 'binary'
    };
    const assetRoot = path.join(root, 'asset-cases'); await writeFiles(assetRoot, assetFiles);
    const assetCases = [];
    for (const [file, extension, type] of [['style.css', '.css', 'css'], ['image.svg', '.svg', 'misc'], ['audio.wav', '.wav', 'sound'], ['audio.mp3', '.mp3', 'sound'], ['audio.m4a', '.m4a', 'sound'], ['blob.bin', '.bin', 'misc']]) {
      const t = { id: 'asset', name: 'asset', folderPath: assetRoot, assets: { asset: { path: path.join(assetRoot, file), relativePath: file, extension, type } } } as Theme;
      assetCases.push(normalize({ theme: t, kind: 'asset', name: 'asset', result: await loadThemeAssetData(t, 'asset') }));
    }
    for (const cssFile of ['style.css', '.css', '../outside.css', 'C:\\evil.css']) {
      const t = { id: 'css', name: 'css', folderPath: assetRoot, cssFile };
      assetCases.push(normalize({ theme: t, kind: 'css', name: cssFile, result: await loadThemeCssFile(t, cssFile) }));
    }
    const soundVolumes = [];
    for (const [tag, volume] of [['NaN', NaN], ['Infinity', Infinity], ['-Infinity', -Infinity], ['50', 50], ['-10', -10], ['150', 150]] as const) {
      for (const event of ['undo', 'redo', 'taskCompleted', 'backgroundActivated'] as const) {
        const t = { id: 's', name: 's', sounds: { events: { [event]: 'audio.wav' } } } as Theme;
        let actual: number | null = null;
        const controller = createThemeSoundController({
          api: { async getThemeAsset() { return { data: 'YQ==', mimeType: 'audio/wav', extension: '.wav', type: 'sound', isText: false }; } },
          createAudio() { return { currentTime: 0, volume: 0, play() { actual = this.volume; } }; }
        });
        controller.applyTheme(t, { enabled: true, volume }); await controller.play(event);
        soundVolumes.push({ theme: t, event, settingsPercent: tag, volume: actual });
      }
    }
    const resolvedSounds = [];
    for (const event of ['undo', 'redo', 'taskCompleted', 'backgroundActivated'] as const) {
      const t = { id: 'sound-assets', name: 'sound-assets', folderPath: assetRoot,
        sounds: { volume: .8, events: { undo: 'audio.wav', redo: 'missing.ogg', taskCompleted: 'image.svg', backgroundActivated: { file: 'audio.wav', volume: .5 } } },
        assets: { audio: { path: path.join(assetRoot, 'audio.wav'), relativePath: 'audio.wav', type: 'sound', extension: '.wav' }, image: { path: path.join(assetRoot, 'image.svg'), relativePath: 'image.svg', type: 'misc', extension: '.svg' }, missing: { path: path.join(assetRoot, 'missing.ogg'), relativePath: 'missing.ogg', type: 'sound', extension: '.ogg' } }
      } as Theme;
      const played: { source: string; volume: number }[] = [];
      const controller = createThemeSoundController({
        api: { async getThemeAsset(_id, asset) { return loadThemeAssetData(t, asset); } },
        createAudio(source) { return { currentTime: 0, volume: 0, play() { played.push({ source, volume: this.volume }); } }; },
        logger: { warn() {}, error() {} }
      });
      controller.applyTheme(t, { enabled: true, volume: 50 }); await controller.play(event);
      resolvedSounds.push(normalize({ theme: t, event, played }));
    }
    // Separate observation, not a parity case: JS prototype-backed assets cannot
    // be represented faithfully by a plain JSON map. Run the unchanged library
    // and delivery/sound pipeline; never emulate prototype mutation here.
    const prototypeFiles: Files = {
      'folder/theme.json': theme('prototype-asset', {
        sounds: { events: { undo: '__proto__.wav' } }, variants: { inherited: {}, override: {} }
      }),
      'folder/__proto__.wav': 'base audio',
      'folder/variants/override/__proto__.wav': 'variant audio'
    };
    const prototypeDir = path.join(root, 'prototype-assets');
    const prototypeThemes = path.join(prototypeDir, 'themes');
    const prototypeDefaults = path.join(prototypeDir, 'defaults');
    await writeFiles(prototypeThemes, prototypeFiles); await fs.mkdir(prototypeDefaults, { recursive: true });
    const prototypeLibrary = new ThemeLibrary({ dataDir: prototypeDir, themesDir: prototypeThemes,
      defaultThemesDir: prototypeDefaults, isPackaged: false, platform: 'linux',
      logger: { log() {}, warn() {}, error() {} } });
    await prototypeLibrary.initialize();
    const prototypeObservations = [];
    for (const t of prototypeLibrary.themes.values()) {
      const played: { source: string; volume: number }[] = [];
      const controller = createThemeSoundController({
        api: { async getThemeAsset(_id, asset) { return loadThemeAssetData(t, asset); } },
        createAudio(source) { return { currentTime: 0, volume: 0, play() { played.push({ source, volume: this.volume }); } }; },
        logger: { warn() {}, error() {} }
      });
      controller.applyTheme(t, { enabled: true, volume: 50 }); await controller.play('undo');
      prototypeObservations.push(normalize({ theme: t, ownAssetNames: Object.keys(t.assets || {}),
        hasOwnPrototypeAsset: Object.prototype.hasOwnProperty.call(t.assets, '__proto__'),
        prototypeIsAsset: Object.getPrototypeOf(t.assets)?.relativePath?.endsWith('/__proto__.wav') === true,
        delivery: await loadThemeAssetData(t, '__proto__'), played }));
    }
    const prototypeAssetBoundary = { status: 'unresolved-compatibility-bound-not-an-accepted-delta',
      files: prototypeFiles, observations: prototypeObservations };
    return { stamp, provenance: await provenance(), cases, sounds, assetFiles, assetCases, soundVolumes, resolvedSounds, prototypeAssetBoundary };
  } finally { vi.useRealTimers(); await fs.rm(root, { recursive: true, force: true }); }
}

test('freshness rejects copied source and dependency mutations without rewriting the oracle', async () => {
  const expected = await provenance();
  const root = await fs.mkdtemp(path.join(process.env.TMPDIR || os.tmpdir(), 'theme-library-provenance-'));
  try {
    for (const file of Object.keys(expected)) {
      await fs.mkdir(path.dirname(path.join(root, file)), { recursive: true });
      await fs.copyFile(file, path.join(root, file));
    }
    expect(await provenance(root)).toEqual(expected);
    for (const file of ['src/main/themes/themeLibrary.ts', 'node_modules/defu/dist/defu.mjs', 'node_modules/destr/dist/index.cjs']) {
      await fs.appendFile(path.join(root, file), '\n// mutation probe\n');
      const probe = await provenance(root);
      expect(() => expect(probe).toEqual(expected)).toThrow();
      await fs.copyFile(file, path.join(root, file));
    }
    expect(await provenance()).toEqual(expected);
  } finally { await fs.rm(root, { recursive: true, force: true }); }
});

test('real TypeScript filesystem oracle is fresh, deterministic, and leaves bundled sources untouched', async () => {
  const before = await provenance();
  if (process.env.THEME_LIBRARY_GOLDEN_CHECK) expect(JSON.parse(await fs.readFile(output, 'utf8')).provenance).toEqual(before);
  const first = JSON.stringify(await generate(), null, 2) + '\n';
  expect(JSON.stringify(await generate(), null, 2) + '\n').toBe(first);
  expect(await provenance()).toEqual(before);
  if (process.env.THEME_LIBRARY_GOLDEN_CHECK) expect(await fs.readFile(output, 'utf8')).toBe(first);
  else { await fs.mkdir(path.dirname(output), { recursive: true }); await fs.writeFile(output, first); }
});
