import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';
import { test, expect, vi } from 'vitest';
import { parseJson, writeJsonFile } from '../../src/main/utils/jsonStorage';
import { registerStorageIpc } from '../../src/main/ipc/storageIpc';
import { initializeRuntimeData } from '../../src/main/storage/runtimeDataBootstrap';
import type { Settings } from '../../src/shared/types';

const stamp = '2026-07-24T12:34:56.789Z';
const suffix = '.corrupt-2026-07-24T12-34-56-789Z';
const names = ['current_tasks.json', 'templates.json', 'settings.json'];
const kinds = ['tasks', 'templates', 'settings'];
const logger = { log() {}, error() {}, warn() {} };

function harness(root: string) {
  const handlers = new Map<string, (...args: unknown[]) => unknown>();
  registerStorageIpc({ handle(channel: string, handler: (...args: unknown[]) => unknown) {
    handlers.set(channel, handler);
  } } as unknown as Parameters<typeof registerStorageIpc>[0], {
    tasksFile: path.join(root, names[0]), templatesFile: path.join(root, names[1]),
    settingsFile: path.join(root, names[2])
  }, logger);
  return async (channel: string, ...args: unknown[]) => handlers.get(channel)!({}, ...args);
}

test('generate storage fixtures from the actual TypeScript implementation', async () => {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'echosight-storage-oracle-'));
  vi.useFakeTimers({ toFake: ['Date'] });
  vi.setSystemTime(new Date(stamp));
  try {
    const inputs = [
      '', '   ', '{', '{"tasks":[', 'hello', 'true', ' TRUE ', 'FaLsE', ' NULL ',
      '\uFEFFtrue', '\uFEFF{}', '\u00a0null', '\u00a0{}', 'undefined', 'UNDEFINED',
      'NaN', 'Infinity', '-Infinity', '1e400', '0', '-0', '1.25',
      '1749938954072.5596', '12345678901234567', '1234567890123456',
      '0.123456789012345678', '01', '+1', '.5', '1.', '1e-7', '1e21',
      '"raw\nline"', '"inner"quote"', ' "normal" ', '"escaped\\nline"',
      '"\\ud800"', '[1,true,null]', '{"b":1,"2":2,"1":1,"a":0}',
      '{"__proto__":{}}', '{"constructor":1}', '{"x":{"constructor":null}}',
      '{"\\u005f\\u005fproto__":1}', '{"\\u0063onstructor":1}',
      '{"Constructor":1}', '{"text":"__proto__"}', '{"large":9007199254740993}',
      '{"a":1,"a":2}', '{"x":1e400}', '["\\ud800"]'
    ];
    const parse = inputs.map(input => {
      try {
        const value = parseJson(input);
        if (value === undefined) return { input, status: 'unsupported', reason: 'undefined' };
        if (typeof value === 'number' && !Number.isFinite(value)) {
          return { input, status: 'unsupported', reason: String(value) };
        }
        // serde_json::Value cannot represent unpaired UTF-16 surrogates.
        if (JSON.stringify(value).includes('\\ud800')) {
          return { input, status: 'unsupported', reason: 'unpaired surrogate' };
        }
        if (input === '{"x":1e400}') return { input, status: 'unsupported', reason: 'nested nonfinite' };
        return { input, status: 'ok', pretty: JSON.stringify(value, null, 2) };
      } catch {
        return { input, status: 'invalid' };
      }
    });
    const values = [
      { tasks: [{ id: 1749938954072.5596, text: '雪 👁\n\t\u0001', completed: false }], currentTemplate: null, snapshots: [] },
      { b: 1, '10': 'ten', '2': 'two', '01': 'not index', '4294967295': 'not index', a: {} },
      [0, -0, 1, 1.5, 1e-7, 1e-6, 1e20, 1e21, 333333333.3333333, 0.0000123456789012345],
      { quote: '"\\', line: '\u2028\u2029', nested: [[], {}, null] }, null, [], {}, 'text', true
    ];
    const writes = [];
    for (const [i, value] of values.entries()) {
      const target = path.join(root, `write-${i}.json`);
      await writeJsonFile(target, value);
      writes.push({ value, bytes: await fs.readFile(target, 'utf8') });
    }
    const recovery = [];
    const missing = [];
    for (const [i, kind] of kinds.entries()) {
      const dir = path.join(root, kind);
      await fs.mkdir(dir);
      const invoke = harness(dir);
      missing.push({ kind, value: await invoke(`load-${kind}`), files: await fs.readdir(dir) });
      for (const [j, bytes] of [Buffer.alloc(0), Buffer.from('{"tasks":['), Buffer.from([0xff, 0xfe, 0x7b])].entries()) {
        const file = path.join(dir, names[i]);
        await fs.writeFile(file, bytes);
        const value = await invoke(`load-${kind}`);
        if (kind === 'tasks') (value as Record<string, unknown>).corruptBackupPath = names[i] + suffix;
        recovery.push({ kind, case: j, input: [...bytes], value,
          backup: [...await fs.readFile(file + suffix)], live: await fs.readFile(file, 'utf8') });
      }
    }
    const settings = { settingsVersion: 2, transparency: 70, theme: 'echosight', hotkeys: {
      toggleVisibility: 'CommandOrControl+Shift+T', toggleInteractive: 'CommandOrControl+Shift+I',
      completeNextTask: 'CommandOrControl+Shift+N', undoLastAction: 'CommandOrControl+Shift+Z',
      redoLastAction: 'CommandOrControl+Shift+Y'
    }, sounds: { enabled: false, volume: 60 } } satisfies Settings;
    const bootstrap = [];
    for (const scenario of ['bundled', 'missing', 'corrupt', 'existing']) {
      const dataDir = path.join(root, 'bootstrap', scenario, 'data');
      const defaultDataDir = path.join(root, 'bootstrap', scenario, 'defaults');
      await fs.mkdir(defaultDataDir, { recursive: true });
      const sources: Record<string, string> = scenario === 'bundled' ? {
        'current_tasks.json': '{"tasks":[{"text":"Welcome","id":1749938954072.5596}],"currentTemplate":"Starter"}',
        'templates.json': '[{"name":"Starter","tasks":[]}]', 'settings.json': '{"theme":"DO NOT COPY"}'
      } : scenario === 'corrupt' ? { 'current_tasks.json': '{', 'templates.json': '' } : {};
      for (const [name, bytes] of Object.entries(sources)) await fs.writeFile(path.join(defaultDataDir, name), bytes);
      const existing: Record<string, string> = scenario === 'existing' ? {
        'current_tasks.json': '{CORRUPT but preserve', 'templates.json': '', 'settings.json': '{"theme":"existing"}'
      } : {};
      await fs.mkdir(dataDir, { recursive: true });
      for (const [name, bytes] of Object.entries(existing)) await fs.writeFile(path.join(dataDir, name), bytes);
      await initializeRuntimeData({ dataDir, defaultDataDir, settings, logger });
      const files: Record<string, string> = {};
      for (const name of names) files[name] = await fs.readFile(path.join(dataDir, name), 'utf8');
      bootstrap.push({ scenario, sources, existing, settings, files });
    }
    const sources = ['src/main/utils/jsonStorage.ts', 'src/main/ipc/storageIpc.ts', 'src/main/storage/runtimeDataBootstrap.ts', 'node_modules/destr/dist/index.mjs'];
    const hashes: Record<string, string> = {};
    for (const source of sources) hashes[source] = createHash('sha256').update(await fs.readFile(source)).digest('hex');
    const output = { provenance: { sourceHashes: hashes, destrVersion: JSON.parse(await fs.readFile('node_modules/destr/package.json', 'utf8')).version }, parse, writes, missing, recovery, bootstrap };
    const dest = 'crates/echosight-core/tests/golden/storage';
    await fs.mkdir(dest, { recursive: true });
    await fs.writeFile(path.join(dest, 'storage.json'), JSON.stringify(output, null, 2) + '\n');
    expect(writes.length).toBe(values.length);
  } finally {
    vi.useRealTimers();
    await fs.rm(root, { recursive: true, force: true });
  }
});
