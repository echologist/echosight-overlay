// Oracle: execute the shipped TypeScript. Never hand-edit expected JSON.
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { afterEach, expect, test, vi } from 'vitest';
import type { EchosightApi, Task, TaskLoadData, TaskSaveData } from '../../src/shared/types';
import { createTaskStateController } from '../../src/renderer/features/tasks/taskStateController';
import { migrateTaskStructure } from '../../src/renderer/features/tasks/taskMigration';
import { sanitizeBackgroundOptions } from '../../src/renderer/features/tasks/taskCreation';
import { findParentTask, findTaskWithParent, flattenTasks } from '../../src/renderer/features/tasks/taskTree';
import { hasIncompleteTasks } from '../../src/renderer/features/tasks/taskCompletion';

const epoch = 1_800_000_000_000;
const out = resolve('crates/echosight-core/tests/golden/tasks');
const clone = <T>(x: T): T => JSON.parse(JSON.stringify(x));
const task = (id: number, text: string, extra: Partial<Task> = {}): Task => ({
  id, text, completed: false, createdAt: '2026-04-27T00:00:00.000Z', children: [],
  mode: 'main', triggers: [], activated: true, activatedAt: null, backgroundOptions: null, ...extra
});
type Op = { op: string; nowMs?: number; random?: number; [key: string]: unknown };
type Scenario = { name: string; tasks: Task[]; currentTemplate?: string | null; snapshots?: unknown[]; ops: Op[] };
const nested = () => [task(1, 'Root', { children: [task(2.5, 'Branch', {
  children: [task(3, 'Leaf A'), task(4, 'Leaf B')]
}), task(5, 'Leaf C')] }), task(6, 'Other')];
const scenarios: Scenario[] = [
  { name: 'complete-next-duplicate-root-ids', tasks: [], ops: [
    { op: 'add', text: 'First' }, { op: 'add', text: 'Second' },
    { op: 'next' }, { op: 'next' }, { op: 'next' },
    { op: 'undo' }, { op: 'redo' }, { op: 'save' }
  ].map(op => ({ ...op, nowMs: epoch, random: 0.017 })) },
  { name: 'complete-next-duplicate-nested-ids', tasks: [
    task(1, 'Root', { children: [task(2, 'First branch'), task(3, 'Second branch')] })
  ], ops: [
    { op: 'add', text: 'First', parent: 2 }, { op: 'add', text: 'Second', parent: 3 },
    { op: 'next' }, { op: 'next' }, { op: 'next' },
    { op: 'undo' }, { op: 'redo' }, { op: 'save' }
  ].map(op => ({ ...op, nowMs: epoch, random: 0.017 })) },
  { name: 'creation-and-inheritance', tasks: [], ops: [
    { op: 'add', text: '  verbatim  ', as: 'root' }, { op: 'add', text: 'child', parent: 'root' },
    { op: 'add', text: 'missing parent', parent: 999 },
    { op: 'background', text: 'Dormant', options: { expiresAfterMinutes: '2.9', priority: 'high' }, as: 'bg' },
    { op: 'add', text: 'Dormant child', parent: 'bg' }, { op: 'undo' }, { op: 'redo' },
    { op: 'save' }
  ] },
  { name: 'nested-complete-next', tasks: nested(), ops: [
    { op: 'find', id: 2.5 }, { op: 'location', id: 3 }, { op: 'parent', id: 3 },
    { op: 'flatten' }, { op: 'incomplete' }, ...Array.from({ length: 5 }, () => ({ op: 'next' })),
    { op: 'incomplete' }, { op: 'undo' }, { op: 'redo' }, { op: 'save' }
  ] },
  { name: 'toggle-parent-propagation', tasks: nested(), ops: [
    { op: 'toggle', id: 1 }, { op: 'toggle', id: 3 }, { op: 'toggle', id: 3 },
    { op: 'toggle', id: 1 }, { op: 'next' }, { op: 'toggle', id: 999 },
    { op: 'undo' }, { op: 'redo' }
  ] },
  { name: 'completion-root-filter-quirks', tasks: [
    task(1, 'Done ancestor', { completed: true, children: [task(2, 'Not visited')] }),
    task(3, 'Incomplete parent with done children', { children: [task(4, 'Done leaf', { completed: true })] }),
    task(5, 'Background root', { mode: 'background', activated: false, children: [task(6, 'Main child')] }),
    task(7, 'Main root', { children: [task(8, 'Nested background', { mode: 'background', children: [task(9, 'Main grandchild')] })] })
  ], ops: [{ op: 'next' }, { op: 'next' }, { op: 'incomplete' }, { op: 'toggle', id: 5 }, { op: 'next' }] },
  { name: 'progress-root-filter', tasks: [
    task(1, 'Main', { children: [task(2, 'Dormant nested background', { mode: 'background', activated: false, completed: true })] }),
    task(3, 'Dormant root', { mode: 'background', activated: false, children: [task(4, 'Ignored child', { completed: true })] }),
    task(5, 'Active root', { mode: 'background', activated: true, children: [task(6, 'Dormant counted child', { mode: 'background', activated: false })] })
  ], ops: [{ op: 'progress' }, { op: 'toggle', id: 6 }, { op: 'next' }, { op: 'incomplete' }] },
  { name: 'delete-descendants-and-dangling-triggers', tasks: [
    ...nested(), task(20, 'Watcher', { mode: 'background', activated: false, triggers: [1, 2.5, 3, 4, 999], children: [task(21, 'Nested watcher', { triggers: [2.5, 4, 6] })] })
  ], ops: [{ op: 'delete', id: 2.5 }, { op: 'undo' }, { op: 'redo' },
    { op: 'delete', id: 999 }, { op: 'undo' }, { op: 'redo' }, { op: 'delete', id: 1 }] },
  { name: 'edit-preserves-state', tasks: [task(1, 'Parent', { children: [task(2, 'Old child', {
    completed: true, triggers: [20], activated: false, activatedAt: '2026-04-27T02:00:00.000Z',
    backgroundOptions: { expiresAfterMinutes: 10, priority: 'normal' }
  })] }), task(20, 'Old background', { completed: true, mode: 'background', triggers: [30],
    activatedAt: '2026-04-27T02:00:00.000Z', children: [task(21, 'Background child')],
    backgroundOptions: { expiresAfterMinutes: 15, priority: 'normal' }
  })], ops: [{ op: 'edit', id: 2, text: '  Renamed child  ', priority: 'high' },
    { op: 'edit', id: 20, text: '\ufeff New background \u00a0', priority: 'high' },
    { op: 'undo' }, { op: 'redo' }, { op: 'edit', id: 20, text: 'New background', priority: 'normal' },
    { op: 'undo' }, { op: 'save' }] },
  { name: 'noop-and-redo-invalidation', tasks: [task(1, 'Keep me'), task(2, 'BG', { mode: 'background' })], ops: [
    { op: 'edit', id: 1, text: '   ' }, { op: 'edit', id: 999, text: 'Missing' },
    { op: 'edit', id: 1, text: ' Keep me ', priority: 'high' }, { op: 'undo' },
    { op: 'toggle', id: 1 }, { op: 'undo' }, { op: 'edit', id: 1, text: 'Keep me' }, { op: 'redo' },
    { op: 'undo' }, { op: 'add', text: 'New task' }, { op: 'redo' },
    { op: 'edit', id: 2, text: 'BG', priority: 'high' }, { op: 'undo' }, { op: 'redo' }
  ] },
  { name: 'moves-and-guards', tasks: [...nested(), task(20, 'BG', { mode: 'background' })], ops: [
    { op: 'move', id: 1, target: 3, above: false, subtask: true },
    { op: 'move', id: 1, target: 20, above: false },
    { op: 'move', id: 999, target: 1, above: true },
    { op: 'move', id: 1, target: 999, above: true },
    { op: 'move', id: 1, target: 1, above: false }, { op: 'undo' },
    { op: 'move', id: 6, target: 1, above: true },
    { op: 'move', id: 6, target: 2.5, above: false },
    { op: 'move', id: 3, target: 1, above: false },
    { op: 'move', id: 3, target: 6, above: false, subtask: true },
    { op: 'undo' }, { op: 'redo' }, { op: 'location', id: 3 }, { op: 'location', id: 999 }
  ] },
  { name: 'self-subtask-source-quirk', tasks: [task(1, 'Self'), task(2, 'Other')], ops: [
    { op: 'move', id: 1, target: 1, above: false, subtask: true }, { op: 'undo' }, { op: 'redo' }
  ] },
  { name: 'history-20-actions', tasks: [task(1, 'Counter')], ops: [
    ...Array.from({ length: 23 }, (_, i) => ({ op: 'edit', id: 1, text: `Edit ${i}` })),
    ...Array.from({ length: 22 }, () => ({ op: 'undo' })),
    ...Array.from({ length: 22 }, () => ({ op: 'redo' })), { op: 'save' }
  ] },
  { name: 'snapshots-five-and-deep-clones', tasks: [task(1, 'Snapshot target')], currentTemplate: 'Mapping',
    snapshots: Array.from({ length: 7 }, (_, i) => ({ createdAt: `legacy-${i}`, state: { tasks: [], currentTemplate: `Old ${i}` } })), ops: [
      { op: 'save' }, ...Array.from({ length: 6 }, (_, i) => [
        { op: 'edit', id: 1, text: `Snapshot ${i}` }, { op: 'save' }
      ]).flat(), { op: 'undo' }, { op: 'save' }
    ] },
  { name: 'clear-replace-and-explicit-capture', tasks: [], currentTemplate: 'Empty template', ops: [
    { op: 'clear' }, { op: 'undo' }, { op: 'save' }, { op: 'redo' }, { op: 'clear' }, { op: 'undo' },
    { op: 'capture', label: 'configure triggers' },
    { op: 'replace', tasks: nested(), currentTemplate: 'Replacement' },
    { op: 'undo' }, { op: 'save' }, { op: 'redo' }, { op: 'save' }, { op: 'clear' }, { op: 'undo' }
  ] },
  { name: 'fractional-legacy-id', tasks: [task(1749938954072.5596, 'Legacy')], ops: [
    { op: 'find', id: 1749938954072.5596 }, { op: 'edit', id: 1749938954072.5596, text: 'Still fractional' },
    { op: 'toggle', id: 1749938954072.5596 }, { op: 'undo' }, { op: 'save' }
  ] }
];
// Deterministic 30-step mixed sequence; inputs only, outcomes come from TS.
let seed = 0x12345678;
const random = () => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed; };
scenarios.push({ name: 'seeded-30-step-sequence', tasks: nested(), ops: Array.from({ length: 30 }, (_, i) => {
  const id = [1, 2.5, 3, 4, 5, 6, 999][random() % 7];
  switch (random() % 7) {
    case 0: return { op: 'toggle', id };
    case 1: return { op: 'edit', id, text: `Seed edit ${i}` };
    case 2: return { op: 'next' };
    case 3: return { op: 'undo' };
    case 4: return { op: 'redo' };
    case 5: return { op: 'move', id, target: 6, above: true };
    default: return { op: 'save' };
  }
}) });

afterEach(() => { vi.useRealTimers(); vi.restoreAllMocks(); });
function emit(name: string, input: unknown, expected: unknown) {
  mkdirSync(out, { recursive: true });
  writeFileSync(resolve(out, `${name}.input.json`), JSON.stringify(input, null, 2));
  writeFileSync(resolve(out, `${name}.expected.json`), JSON.stringify(expected, null, 2));
}
for (const scenario of scenarios) test(`generate ${scenario.name}`, async () => {
  vi.useFakeTimers();
  vi.setSystemTime(epoch);
  let randomCalls = 0;
  let explicitRandom: number | undefined;
  vi.spyOn(Math, 'random').mockImplementation(() => explicitRandom ?? ((randomCalls++ * 137 + 17) % 1000) / 1000);
  let saved: TaskSaveData | null = null;
  const initial = { tasks: scenario.tasks, currentTemplate: scenario.currentTemplate ?? null, snapshots: scenario.snapshots ?? [] };
  const api = {
    loadTasks: async () => clone(initial) as TaskLoadData,
    saveTasks: async (state: TaskSaveData) => { saved = clone(state); return { success: true }; }
  } as EchosightApi;
  const c = createTaskStateController({ api, logger: { log() {}, error() {} } });
  await c.loadTasks();
  const aliases = new Map<string, number>();
  const id = (v: unknown): number => typeof v === 'string' ? aliases.get(v)! : v as number;
  const frames: unknown[] = [];
  for (const [index, o] of scenario.ops.entries()) {
    vi.setSystemTime(o.nowMs ?? epoch + index * 1000);
    explicitRandom = o.random;
    let result: unknown = null;
    switch (o.op) {
      case 'add': { const r = c.addTask(o.text as string, o.parent == null ? null : id(o.parent)); if (r && o.as) aliases.set(o.as as string, r.task.id); result = r; break; }
      case 'background': { const r = c.addBackgroundTask(o.text as string, o.options); if (o.as) aliases.set(o.as as string, r.id); result = r; break; }
      case 'edit': result = c.editTask(id(o.id), o.text as string, o.priority as 'normal' | 'high' | undefined); break;
      case 'delete': result = c.deleteTask(id(o.id)); break;
      case 'toggle': result = c.toggleTask(id(o.id)); break;
      case 'next': result = c.completeNextTask(); break;
      case 'undo': result = c.undoLastAction(); break;
      case 'redo': result = c.redoLastAction(); break;
      case 'clear': c.clearTasks(); break;
      case 'capture': c.captureUndoState(o.label as string); break;
      case 'replace': c.replaceTasks(clone(o.tasks) as Task[], o.currentTemplate as string | null); break;
      case 'move': result = c.moveTask(id(o.id), id(o.target), !!o.above, !!o.subtask); break;
      case 'find': result = c.findTaskById(id(o.id)); break;
      case 'parent': result = findParentTask(id(o.id), c.getTasks()); break;
      case 'location': { const r = findTaskWithParent(id(o.id), c.getTasks()); result = r ? { task: r.task, parentId: r.parent?.id ?? null, level: r.level, arrayIds: r.array.map(t => t.id) } : null; break; }
      case 'flatten': result = flattenTasks(c.getTasks()).map(t => t.id); break;
      case 'incomplete': result = hasIncompleteTasks(c.getTasks()); break;
      case 'progress': result = c.getProgress(); break;
      case 'save': expect(await c.saveTasks()).toBe(true); result = saved; break;
      default: throw new Error(`Unknown operation ${o.op}`);
    }
    frames.push(clone({ result, tasks: c.getTasks(), progress: c.getProgress() }));
  }
  emit(scenario.name, { kind: 'state', epoch, ...initial, ops: scenario.ops.map((o, i) => ({
    ...o, nowMs: o.nowMs ?? epoch + i * 1000, nowIso: new Date(o.nowMs ?? epoch + i * 1000).toISOString()
  })) }, frames);
});

const migrations: unknown[] = [null, {}, [], [null, false, 2, 'ignored'], [
  { id: 1749938954072.5596, text: 'Legacy', completed: false, createdAt: 'old', extra: { keep: true }, children: [
    { id: 2.5, text: 'Child', children: 'broken', mode: '', triggers: {}, activated: null, activatedAt: null, backgroundOptions: {} }, null, 7
  ] },
  { id: 3, text: 'Background legacy', mode: 'background' }
], [task(1, 'Already migrated')]];
for (const [i, input] of migrations.entries()) test(`generate migration ${i}`, () => {
  const tasks = clone(input);
  const migrated = migrateTaskStructure(tasks);
  const secondMigration = migrateTaskStructure(tasks);
  emit(`migration-${i}`, { kind: 'migration', tasks: input }, { migrated, tasks, secondMigration });
});
test('generate background option coercions', () => {
  const values = [null, [], 'x', {}, { expiresAfterMinutes: '2.9', priority: 'high' },
    ...[0, -1, 0.5, true, false, null, '', '0x10', '0b11', '0o10', '+0x10', '-0x10', 'Infinity', 'inf', '1e2', [], [true], [null], ['2.8'], [[2]], [1, 2], {}]
      .map(expiresAfterMinutes => ({ expiresAfterMinutes, priority: 'HIGH' }))];
  emit('background-options', { kind: 'options', values }, values.map(sanitizeBackgroundOptions));
});
