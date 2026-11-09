// Real shipped TS is the oracle. No Electron, DOM, or existing fixtures are changed.
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { expect, test, vi } from 'vitest';
import * as domain from '../../src/renderer/features/tasks/taskTemplates';
import * as ops from '../../src/renderer/features/templates/templateOperations';
import { COMMUNITY_TEMPLATES, createCommunityTemplateTasks } from '../../src/renderer/features/templates/communityTemplates';
import { createCommunityTemplatePreview } from '../../src/renderer/features/templates/communityTemplatesUi';
import { loadTemplateState, saveTemplateState } from '../../src/renderer/features/templates/templatePersistence';
import type { EchosightApi, Task, TaskTemplate, TemplateTask } from '../../src/shared/types';

const output = 'crates/echosight-core/tests/golden/templates';
const sources = [
  'src/renderer/features/tasks/taskTemplates.ts',
  'src/renderer/features/tasks/taskCreation.ts',
  'src/renderer/features/tasks/taskGuards.ts',
  'src/renderer/features/tasks/taskTree.ts',
  'src/renderer/features/templates/templateOperations.ts',
  'src/renderer/features/templates/templatePersistence.ts',
  'src/renderer/features/templates/communityTemplates.ts',
  'src/renderer/features/templates/communityTemplatesUi.ts',
  'node_modules/destr/dist/index.mjs',
  'tools/golden/templates.gen.test.ts',
  'tools/golden/vitest.templates.config.ts'
];
const nowMs = 1749938954072;
const nowIso = new Date(nowMs).toISOString();
const randoms = [0.123, 0.999, 0, 0.5, 0.5];
const clone = <T>(value: T): T => JSON.parse(JSON.stringify(value));
type Case = { name: string; op: string; input: any; expected: any; randoms: number[] };

function checkedBytes(recorded: string, generated: string): void {
  expect(recorded, 'stale template oracle; regenerate explicitly').toBe(generated);
}
function sourceHashes(): Record<string, string> {
  return Object.fromEntries(sources.map(file => [file,
    createHash('sha256').update(readFileSync(file)).digest('hex')]));
}

test('freshness comparison rejects changed fixture and source bytes', () => {
  expect(() => checkedBytes('old', 'new')).toThrow();
  const hashes = sourceHashes();
  expect(() => checkedBytes(JSON.stringify(hashes), JSON.stringify({ ...hashes,
    [sources[0]]: 'changed source' }))).toThrow();
});

test('generate template normalization, transfer, operations, persistence and community oracles', async () => {
  const cases: Case[] = [];
  let randomIndex = 0;
  let entropy = randoms;
  vi.useFakeTimers();
  vi.setSystemTime(nowMs);
  expect(new Date().toISOString()).toBe(nowIso);
  vi.spyOn(Math, 'random').mockImplementation(() => entropy[randomIndex++ % entropy.length]);
  const add = (name: string, op: string, input: any, run: () => any, repeats = false) => {
    entropy = repeats ? [0.5] : randoms;
    randomIndex = 0;
    // Clone NOW, not after later mutations to referenced tasks/templates.
    let expected: any;
    try { expected = clone(run()); }
    catch (error) { expected = { thrown: { name: (error as Error).name, message: (error as Error).message } }; }
    cases.push({ name, op, input: clone(input), expected, randoms: clone(entropy) });
  };
  const task = (id: number, text: string, extra: Partial<Task> = {}): Task => ({
    id, text, completed: false, createdAt: nowIso, children: [], mode: 'main',
    triggers: [], activated: true, activatedAt: null, backgroundOptions: null, ...extra
  });
  const template = (name: string, tasks: TemplateTask[], id = 1): TaskTemplate => ({
    id, name, tasks, createdAt: nowIso
  });
  try {
    for (const input of [null, false, 7, [], {}, '', '  text  ', '\uFEFFx\u00a0', '\u0085x\u0085',
      { text: 'parent', children: [' child ', null, 42, { text: 'deep', children: ['leaf'] }] },
      { text: 'x', mode: 'background', backgroundOptions: { expiresAfterMinutes: '0.9', priority: 'high' },
        triggerIndices: [0, '1', true, null, false, '', [], [2], [true], {}, -1, 1.5, '1e30', 'Infinity', 0] },
      { text: 'x', mode: 'MAIN', backgroundOptions: true },
      { text: 'x', backgroundOptions: null }, { text: 'x', backgroundOptions: [] },
      { text: 'x', backgroundOptions: {}, children: [], triggerIndices: [] },
      { text: 'x', children: [{ text: ' ', children: ['discarded'] }], extra: 'discarded' }
    ]) add(`normalize task ${JSON.stringify(input)}`, 'normalizeTask', input,
      () => domain.normalizeTemplateTask(input));
    for (const value of [null, false, 0, '3.9', -1, 'Infinity', [], [5]]) {
      const input = { text: 'bg', backgroundOptions: { expiresAfterMinutes: value, priority: 'other' } };
      add(`options ${JSON.stringify(value)}`, 'normalizeTask', input, () => domain.normalizeTemplateTask(input));
    }
    for (const input of [null, [], {}, { name: ' ', tasks: ['one', false, { text: 'two' }] },
      { name: 'x'.repeat(60), tasks: ['y'.repeat(210), '😀'.repeat(100)] }]) {
      add('normalize data ' + JSON.stringify(input), 'normalizeData', input, () => domain.normalizeTemplateData(input));
    }
    for (const id of [null, '', '0x10', '17.5', true, [], [3], {}, 'no', 'Infinity']) {
      const input = { id, name: ' Saved ', tasks: [' a ', ''], createdAt: ' untouched date ', imported: true, community: true };
      add('saved id ' + JSON.stringify(id), 'normalizeSaved', input, () => domain.normalizeSavedTemplate(input));
    }
    for (const input of [null, [], 5, {}, { tasks: [] }, { tasks: ['x'], imported: 1, community: 'true', createdAt: ' ' },
      { id: 0, tasks: ['x'], createdAt: null }, { tasks: ['x'], createdAt: 99 }]) {
      add('saved shape ' + JSON.stringify(input), 'normalizeSaved', input, () => domain.normalizeSavedTemplate(input));
    }
    const collection = [null, { tasks: [] }, { id: null, tasks: ['x'] }, { tasks: ['y'] }, { tasks: ['z'] }];
    for (const input of [null, {}, collection]) add('collection ' + JSON.stringify(input), 'normalizeCollection', input,
      () => domain.normalizeTemplateCollection(input), true);
    // Real Number/production observations, including rejected JSON primitives.
    const numberBits = (number: number) => {
      const view = new DataView(new ArrayBuffer(8));
      view.setFloat64(0, number);
      return view.getBigUint64(0).toString(16).padStart(16, '0');
    };
    const observeNumber = (input: unknown) => {
      const number = Number(input);
      return Number.isNaN(number) ? { kind: 'NaN' }
        : !Number.isFinite(number) ? { kind: number > 0 ? 'Infinity' : '-Infinity' }
        : { kind: 'finite', bits: numberBits(number) };
    };
    const radixValues = [
      0n, (1n << 53n) - 1n, (1n << 53n) + 1n, (1n << 53n) + 3n,
      0x200000000000011n, (1n << 57n) + 16n, (1n << 57n) + 48n,
      (1n << 1000n) + (1n << 947n) + 1n,
      (1n << 1024n) - (1n << 971n),
      (1n << 1024n) - (1n << 970n) - 1n,
      (1n << 1024n) - (1n << 970n), 1n << 1100n
    ];
    for (const [radix, prefix] of [[2, '0b'], [8, '0o'], [16, '0x']] as const) {
      for (const integer of radixValues) {
        const input = prefix + integer.toString(radix);
        add('radix number ' + input, 'number', input, () => observeNumber(input));
        add('radix id ' + input, 'parseNumberId', input, () => {
          const id = ops.parseTemplateId(input);
          return id === null ? null : numberBits(id);
        });
      }
      const invalid = prefix + '1'.repeat(1100) + 'z';
      add('invalid radix suffix ' + prefix, 'number', invalid, () => observeNumber(invalid));
    }
    for (const poison of [{ toString: null }, { toString: 0 }, { toString: {} }, [[{ toString: null }]]]) {
      add('number rejects ' + JSON.stringify(poison), 'number', poison, () => observeNumber(poison));
      const task = { text: 'x', triggerIndices: [poison] };
      const options = { text: 'x', backgroundOptions: { expiresAfterMinutes: poison } };
      for (const input of [task, options]) {
        add('task rejects ' + JSON.stringify(input), 'normalizeTask', input, () => domain.normalizeTemplateTask(input));
        const data = { name: 'x', tasks: [input] };
        add('data rejects ' + JSON.stringify(input), 'normalizeData', data, () => domain.normalizeTemplateData(data));
        const text = JSON.stringify(data);
        add('import rejects ' + text, 'import', { text }, () => {
          try { return { ok: domain.parseTemplateImport(text) }; }
          catch (error) { return { error: (error as Error).message }; }
        });
      }
      const saved = { id: poison, tasks: ['x'] };
      add('saved rejects ' + JSON.stringify(poison), 'normalizeSaved', saved, () => domain.normalizeSavedTemplate(saved));
      add('collection rejects ' + JSON.stringify(poison), 'normalizeCollection', [saved], () => domain.normalizeTemplateCollection([saved]));
    }
    for (const input of [{}, { valueOf: null }, [{ valueOf: 0 }], [null], [[null]], [1, 2]]) {
      add('number accepts ' + JSON.stringify(input), 'number', input, () => observeNumber(input));
    }
    for (const input of ['', ' ', '\uFEFF', '0', '-0', ' 0x10 ', '0b11', '0o10', '1.25', '1e3', 'Infinity', 'NaN', 'no', '\u0085']) {
      add('parse id ' + input, 'parseId', input, () => ops.parseTemplateId(input));
    }
    for (const input of ['Hello World!', ' HC/SSF Priorities ', '---', '你好', 'İSTANBUL', 'A_2.0', 'Σigma', 'Kelvin']) {
      add('filename ' + input, 'filename', input, () => ops.createTemplateExportFilename(input));
    }
    const inputs: { text?: string; pad?: number; suffix?: string }[] = [
      ...['', ' \uFEFF ', 'bad', 'true', 'null', 'undefined', 'NaN', 'Infinity', '1e400', '-1e400', '9.9e999', '1e400x', '[]', '{}',
        '{"name":1,"tasks":["x"]}', '{"name":"x","tasks":{}}', '{"name":"x","tasks":[]}',
        '{"name":"x","tasks":[null,7," "]}', '{"name":" ","tasks":[" a ",{"text":"b","children":["c"]}]}',
        '{"name":"x","tasks":["x"],"__proto__":{}}',
        '{"name":"x","tasks":["x"],"constructor":{}}',
        '\uFEFF {"name":"x","tasks":["x"]} \u00a0',
        '"unescaped"inside"'].map(text => ({ text })),
      { pad: 100000 - JSON.stringify({ name: 'x', tasks: [''] }).length, suffix: 'x' },
      { pad: 100001 - JSON.stringify({ name: 'x', tasks: [''] }).length, suffix: 'x' },
      { pad: Math.ceil((100001 - JSON.stringify({ name: 'x', tasks: [''] }).length) / 2), suffix: '😀' }
    ];
    for (const input of inputs) {
      const text = input.text ?? `{"name":"x","tasks":["${input.suffix!.repeat(input.pad!)}"]}`;
      add('import ' + (input.text ?? JSON.stringify(input)), 'import', input, () => {
        try { return { ok: domain.parseTemplateImport(text) }; }
        catch (error) { return { error: (error as Error).message }; }
      });
    }
    const tasks = [
      task(1, 'main', { children: [task(2, 'child', { children: [task(3, 'leaf', { triggers: [1, 5, 3] })] }),
        task(4, 'completed child', { completed: true, children: [task(40, 'hidden incomplete')] })] }),
      task(5, 'background', { mode: 'background', triggers: [1, 2, 3, 4, 40, 99, 5, 1],
        backgroundOptions: { expiresAfterMinutes: 15, priority: 'high' }, children: [task(6, 'inherited bg')] }),
      task(7, 'completed parent', { completed: true, children: [task(8, 'hidden child')] })
    ];
    for (const input of [[], tasks, [task(1, 'all done', { completed: true })],
      [task(1, 'collision a', { triggers: [1] }), task(1, 'collision b')],
      [task(2, 'bg null', { mode: 'background' })]]) {
      add('strip incomplete ' + cases.length, 'strip', input, () => domain.createTemplateTasks(input));
      add('create from tasks ' + cases.length, 'create', { name: ' Kept verbatim ', tasks: input },
        () => ops.createTemplateFromTasks(' Kept verbatim ', input));
    }
    const nested = domain.createTemplateTasks(tasks);
    nested[0].children![0].children![0].triggerIndices = [0, 2, 999, 1, 0];
    const templates = [template('nested', nested), template('empty', []), template('main', [{ text: 'one' }]),
      template('raw indices', [{ text: 'a', children: [{ text: 'b', triggerIndices: [2, 0] }] },
        { text: 'c', triggerIndices: [-1, 0, 1.5, 99, 1, 1] }]),
      template('normalized fractional duration', [domain.normalizeTemplateTask({ text: 'fraction', mode: 'background', backgroundOptions: { expiresAfterMinutes: 0.9, priority: 'high' } })!]),
      template('null options and ignored main options', [{ text: 'background', mode: 'background', backgroundOptions: null }, { text: 'main', backgroundOptions: { expiresAfterMinutes: 5, priority: 'high' } }]),
      template('background grandchild', [{ text: 'root', children: [{ text: 'bg', mode: 'background', children: [{ text: 'leaf', mode: 'main' }] }] }])];
    for (const input of templates) {
      for (const repeated of [false, true]) add('instantiate ' + input.name + repeated, 'instantiate', input,
        () => ops.instantiateTemplateTasks(input), repeated);
      add('export ' + input.name, 'export', input, () => {
        const data = ops.createTemplateExportData(input);
        return { data, bytes: JSON.stringify(data, null, 2), filename: ops.createTemplateExportFilename(input.name) };
      });
      add('inspect ' + input.name, 'inspect', input.tasks, () => ({
        count: ops.countTemplateTasks(input.tasks), background: domain.hasBackgroundTask(input.tasks),
        triggers: domain.hasTemplateTriggerIndices(input.tasks)
      }));
    }
    const existing = [template('same', [], 1), template('other', [], 2), template('same', [], 3)];
    const replacement = template('same', [{ text: 'new' }], 4);
    add('upsert removes all names and appends', 'upsert', { existing, replacement }, () => ops.upsertTemplateByName(existing, replacement));
    for (const id of [1, 2, 77]) {
      add('remove ' + id, 'remove', { existing, id }, () => ops.removeTemplateById(existing, id));
      add('find id ' + id, 'findId', { existing, id }, () => ops.findTemplateById(existing, id));
    }
    for (const name of ['same', 'Same', 'other', 'missing']) add('find name ' + name, 'findName', { existing, name },
      () => ops.findTemplateByName(existing, name));
    const data = { name: 'Imported', tasks: nested };
    add('imported flag and timestamp', 'createImported', data, () => ops.createImportedTemplate(data));
    add('community catalog, imports, previews', 'community', null, () => COMMUNITY_TEMPLATES.map(entry => ({
      ...entry, preview: createCommunityTemplatePreview(entry),
      template: ops.createCommunityTemplate(entry, createCommunityTemplateTasks(entry))
    })));
    for (const input of [{ load: collection }, { load: {} }, { failure: 'IPC unavailable' }]) {
      randomIndex = 0; entropy = randoms;
      const logs: string[][] = [];
      const logger = { error: (prefix: string, error: Error) => logs.push([prefix, error.message]) };
      const api = { loadTemplates: async () => { if ('failure' in input) throw new Error(input.failure); return input.load; } } as unknown as EchosightApi;
      const result = clone(await loadTemplateState(api, logger));
      cases.push({ name: 'load persistence ' + JSON.stringify(input), op: 'load', input, expected: { result, logs }, randoms });
    }
    for (const input of [{ response: { success: true } }, { response: { success: false, error: 'disk full' } },
      { response: { success: false } }, { response: { success: false, error: '' } }, { failure: 'IPC unavailable' }]) {
      const logs: string[][] = [];
      const logger = { error: (prefix: string, error: Error) => logs.push([prefix, error.message]) };
      const api = { saveTemplates: async () => { if ('failure' in input) throw new Error(input.failure); return input.response; } } as unknown as EchosightApi;
      const result = await saveTemplateState(api, [], logger);
      cases.push({ name: 'save persistence ' + JSON.stringify(input), op: 'save', input, expected: { result, logs }, randoms });
    }
    const syntaxErrors = ['{', '{"name":"x","tasks":["x",]}', '{"name":"x" "tasks":[]}'].map(input => {
      let message = '';
      try { domain.parseTemplateImport(input); }
      catch (error) { message = (error as Error).message; }
      expect(message).not.toBe('');
      return { input, message };
    });
    const unsupported = [
      JSON.stringify({ name: 'x', tasks: ['x'.repeat(199) + '😀'] }),
      JSON.stringify({ name: 'x'.repeat(49) + '😀', tasks: ['a'] }),
      '{"name":"x","tasks":["\\ud800"]}',
      '{"name":"x","tasks":["a"],"unused":1e400}'
    ].map(input => ({ input, outputJson: JSON.stringify(domain.parseTemplateImport(input)) }));
    // The renderer catches normalization exceptions and returns an empty list
    // plus a diagnostic. Keep that observation separate from Rust's fail-closed
    // Result (never a successful empty collection eligible for persistence).
    const loadRejections = [];
    for (const input of [
      [{ id: { toString: null }, tasks: ['x'] }],
      [{ tasks: [{ text: 'x', triggerIndices: [[{ toString: 0 }]] }] }],
      [{ tasks: [{ text: 'x', backgroundOptions: { expiresAfterMinutes: { toString: null } } }] }]
    ]) {
      const logs: string[][] = [];
      const logger = { error: (prefix: string, error: Error) => logs.push([prefix, error.name, error.message]) };
      const api = { loadTemplates: async () => input } as unknown as EchosightApi;
      const result = clone(await loadTemplateState(api, logger));
      loadRejections.push({ input, result, logs });
    }
    const document = { sources: sourceHashes(), nowMs, nowIso, cases, syntaxErrors, unsupported, loadRejections };
    const bytes = JSON.stringify(document, null, 2) + '\n';
    if (process.env.TEMPLATES_GOLDEN_CHECK === '1') checkedBytes(readFileSync(`${output}/cases.json`, 'utf8'), bytes);
    else { mkdirSync(output, { recursive: true }); writeFileSync(`${output}/cases.json`, bytes); }
  } finally { vi.restoreAllMocks(); vi.useRealTimers(); }
});
