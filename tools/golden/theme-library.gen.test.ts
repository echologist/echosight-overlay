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
async function provenance() {
  const files = ['src/main/themes/themeLibrary.ts', 'src/main/themes/themeAssetLoaders.ts', 'src/main/themes/themeHelpers.ts',
    'src/main/themes/themeValidation.ts', 'src/main/utils/pathSafety.ts', 'src/main/utils/jsonStorage.ts', 'src/shared/errors.ts',
    'src/renderer/features/themes/themeSounds.ts', 'node_modules/defu/dist/defu.mjs', 'node_modules/destr/dist/index.mjs'];
  const result: Record<string, string> = {};
  for (const file of files) result[file] = createHash('sha256').update(await fs.readFile(file)).digest('hex');
  for (const [file, bytes] of Object.entries(await tree('data/themes'))) result[`data/themes/${file}`] = createHash('sha256').update(Buffer.from(bytes, 'base64')).digest('hex');
  return result;
}
async function generate() {
  const root = await fs.mkdtemp(path.join(process.env.TMPDIR || os.tmpdir(), 'theme-library-'));
  vi.useFakeTimers({ toFake: ['Date'] }); vi.setSystemTime(new Date(stamp));
  const normalize = (value: unknown) => JSON.parse(JSON.stringify(value).split(root).join('$ROOT'));
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
    return { stamp, provenance: await provenance(), cases, sounds };
  } finally { vi.useRealTimers(); await fs.rm(root, { recursive: true, force: true }); }
}

test('real TypeScript filesystem oracle is fresh, deterministic, and leaves bundled sources untouched', async () => {
  const before = await provenance();
  if (process.env.THEME_LIBRARY_GOLDEN_CHECK) expect(JSON.parse(await fs.readFile(output, 'utf8')).provenance).toEqual(before);
  const first = JSON.stringify(await generate(), null, 2) + '\n';
  expect(JSON.stringify(await generate(), null, 2) + '\n').toBe(first);
  expect(await provenance()).toEqual(before);
  if (process.env.THEME_LIBRARY_GOLDEN_CHECK) expect(await fs.readFile(output, 'utf8')).toBe(first);
  else { await fs.mkdir(path.dirname(output), { recursive: true }); await fs.writeFile(output, first); }
});
