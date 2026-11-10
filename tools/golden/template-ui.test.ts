// Shell decisions executed against unchanged renderer modules; data algorithms
// remain covered by templates.gen.test.ts. No handwritten TS controller oracle.
import { afterEach, expect, test, vi } from 'vitest';
import type { EchosightApi, Task, TaskTemplate } from '../../src/shared/types';
import { createTemplateController } from '../../src/renderer/features/templates/templateController';
import { createCommunityTemplateController } from '../../src/renderer/features/templates/communityTemplateController';
import { COMMUNITY_TEMPLATES } from '../../src/renderer/features/templates/communityTemplates';
import { createTaskStateController } from '../../src/renderer/features/tasks/taskStateController';
import { createBackgroundTaskController } from '../../src/renderer/features/tasks/backgroundTaskController';
import * as templateUi from '../../src/renderer/features/templates/templateUi';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

const task = (id: number, overrides: Partial<Task> = {}): Task => ({id, text: `Task ${id}`, completed: false, createdAt: '2027-01-15T08:00:00.000Z', children: [], mode: 'main', triggers: [], activated: true, activatedAt: null, backgroundOptions: null, ...overrides});
const template = (name = 'Saved'): TaskTemplate => ({id: 10, name, createdAt: 'x', tasks: [{text: 'parent', children: [{text: 'child', triggerIndices: [2]}]}, {text: 'background', mode: 'background', backgroundOptions: {priority: 'high', expiresAfterMinutes: 1}}]});
const input = (id: string, text: string) => { (document.getElementById(id) as HTMLInputElement).value = text; };
afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); document.body.replaceChildren(); });
function runtime(initial: TaskTemplate[] = [], tasks = [task(1)]) {
  vi.useFakeTimers(); vi.setSystemTime(1_800_000_000_000); vi.spyOn(Math, 'random').mockReturnValue(0.017);
  document.body.innerHTML = '<select id="templateSelect"></select><div id="saveTemplateModal"><input id="templateNameInput"></div><div id="importTemplateModal"><textarea id="importTemplateInput"></textarea><input id="importFileInput" type="file"></div><div id="communityTemplatesModal"><div id="communityTemplatesList"></div></div>';
  let templates = initial;
  const events: string[] = [];
  const saveTemplates = vi.fn(async (_: TaskTemplate[]) => { events.push('saveTemplates'); return {success: true}; });
  const saveTasks = vi.fn(async () => { events.push('saveTasks'); return {success: true}; });
  const api = {saveTemplates, saveTasks} as unknown as EchosightApi;
  const logger = {log() {}, error() {}};
  const state = createTaskStateController({api, logger}); state.replaceTasks(tasks, null);
  const background = createBackgroundTaskController({getTasks: state.getTasks, onChanged() {}, onActivated() {}, logger});
  const confirm = vi.fn(async () => true); const alert = vi.fn(async () => {});
  const controller = createTemplateController({api, logger, alertUser: alert, confirmUser: confirm,
    getTemplates: () => templates, setTemplates: value => { events.push('publish'); templates = value; }, getTasks: state.getTasks,
    clearBackgroundTimers: () => {events.push('cancelAll'); background.clearAllExpirationTimers();},
    captureUndoState: label => { events.push(label); state.captureUndoState(label); },
    replaceTasks: (next, name) => {events.push('replace'); state.replaceTasks(next, name); state.migrateTaskStructure();},
    onTasksChanged: () => {events.push('render');}, saveTasks: state.saveTasks});
  controller.updateTemplateSelect();
  return {controller, state, background, confirm, alert, saveTemplates, events, templates: () => templates, select: () => input('templateSelect', '10')};
}

test('source provenance is available and actual controller modules execute', () => {
  for (const file of ['templateController.ts', 'templateTransferController.ts', 'communityTemplateController.ts', 'templateUi.ts']) {
    const path = `src/renderer/features/templates/${file}`;
    const sha = createHash('sha256').update(readFileSync(path)).digest('hex');
    expect(sha).toHaveLength(64); console.log(`${path} sha256=${sha}`);
  }
});
test('save silently overwrites by name, waits for successful persistence then resets select', async () => {
  const r = runtime([template()]); r.select(); input('templateNameInput', ' Saved ');
  await r.controller.saveTemplate();
  expect(r.confirm).not.toHaveBeenCalled(); expect(r.templates()).toHaveLength(1);
  expect(r.events).toEqual(['saveTemplates', 'publish']);
  expect((document.getElementById('templateSelect') as HTMLSelectElement).value).toBe('');
  expect(r.templates()[0].name).toBe('Saved');
});
test('failed save leaves published collection, selected option and modal draft intact', async () => {
  const old = template(); const r = runtime([old]); r.select(); input('templateNameInput', 'Saved');
  r.saveTemplates.mockResolvedValue({success: false}); await r.controller.saveTemplate();
  expect(r.templates()).toEqual([old]); expect(r.saveTemplates).toHaveBeenCalledTimes(1); expect(r.events).not.toContain('publish');
  expect((document.getElementById('templateNameInput') as HTMLInputElement).value).toBe('Saved');
  expect(r.alert).toHaveBeenCalledWith('Failed to save template. Please try again.');
});
test('load confirms incomplete descendants, cancels captured timers before replace, captures history, saves without restart', async () => {
  const r = runtime([template()], [task(1, {completed: true, children: [task(2)]})]); r.select();
  const restart = vi.spyOn(r.background, 'restartExpirationTimers');
  await r.controller.loadTemplate();
  expect(r.confirm).toHaveBeenCalledWith('This will replace your current tasks. Continue?', {title: 'Load Template', tone: 'danger'});
  expect(r.events).toEqual(['cancelAll', 'load template', 'replace', 'render', 'saveTasks']);
  expect(restart).not.toHaveBeenCalled();
  const tasks = r.state.getTasks(); expect(tasks[0].children[0].triggers).toEqual([tasks[1].id]);
  expect(tasks[1].activated).toBe(false);
  expect(r.state.undoLastAction().restored).toBe(true);
  expect(r.state.getTasks()[0].id).toBe(1);
});
test('completed-only load needs no confirmation; declined load changes nothing', async () => {
  const r = runtime([template()], [task(1, {completed: true})]); r.select(); await r.controller.loadTemplate(); expect(r.confirm).not.toHaveBeenCalled();
  const r2 = runtime([template()]); r2.select(); r2.confirm.mockResolvedValue(false); await r2.controller.loadTemplate(); expect(r2.events).toEqual([]);
});
test('delete cancel preserves state; confirm persists before publication', async () => {
  const r = runtime([template()]); r.select(); r.confirm.mockResolvedValue(false); await r.controller.deleteTemplate(); expect(r.events).toEqual([]);
  expect(r.confirm).toHaveBeenCalledWith('Are you sure you want to delete the template "Saved"? This cannot be undone.', {title: 'Delete Template', tone: 'danger'});
  r.confirm.mockResolvedValue(true); await r.controller.deleteTemplate(); expect(r.templates()).toEqual([]); expect(r.events).toEqual(['saveTemplates','publish']);
});
test('import exact replacement prompt; cancel consumes no creation or save; invalid draft stays', async () => {
  const r = runtime([template()]); input('importTemplateInput', JSON.stringify({name: 'Saved', tasks: [{text:'one'}]}));
  r.confirm.mockResolvedValue(false); await r.controller.importTemplate();
  expect(r.confirm).toHaveBeenCalledWith('Template "Saved" already exists. Replace it?'); expect(r.events).toEqual([]);
  r.confirm.mockResolvedValue(true); await r.controller.importTemplate(); expect(r.templates()[0].imported).toBe(true);
  input('importTemplateInput', ''); await r.controller.importTemplate(); expect(r.alert).toHaveBeenLastCalledWith('Please paste template JSON, share code, or select a file!');
});
test('export controller delegates actual filename and v2 data; no selection only alerts', () => {
  const r = runtime([template('BG / Name')]);
  const download = vi.spyOn(templateUi, 'downloadJsonFile').mockImplementation(() => {});
  r.controller.exportTemplate(); expect(download).not.toHaveBeenCalled();
  r.select(); r.controller.exportTemplate();
  expect(download).toHaveBeenCalledWith('poe2-template-bg-name.json', expect.objectContaining({name: 'BG / Name', version: '2.0', taskCount: 3, exportedAt: '2027-01-15T08:00:00.000Z'}));
});
test('file import reads actual File contents into draft only; canceled selection preserves draft', async () => {
  const r = runtime(); vi.useRealTimers();
  input('importTemplateInput', 'previous');
  const control = document.getElementById('importFileInput') as HTMLInputElement;
  await r.controller.handleFileImport({target: control} as unknown as Event);
  expect((document.getElementById('importTemplateInput') as HTMLTextAreaElement).value).toBe('previous');
  const text = JSON.stringify({name: 'from file', tasks: [{text:'one'}]}, null, 2);
  Object.defineProperty(control, 'files', {value: [new File([text], 'template.json', {type:'application/json'})]});
  await r.controller.handleFileImport({target: control} as unknown as Event);
  expect((document.getElementById('importTemplateInput') as HTMLTextAreaElement).value).toBe(text);
  expect(r.events).toEqual([]);
});
test('selected zero is ignored; duplicate IDs resolve first matching saved template', async () => {
  const zero = {...template('zero'), id: 0};
  const r = runtime([zero, template('first'), template('second')]);
  input('templateSelect', '0'); await r.controller.loadTemplate(); expect(r.events).toEqual([]);
  r.select(); await r.controller.loadTemplate();
  expect(r.state.getTasks()[0].text).toBe('parent');
  expect(r.events).toEqual(['cancelAll', 'load template', 'replace', 'render', 'saveTasks']);
});

test('all six community imports use source data, retain modal, confirm duplicate and preview numbered tasks', async () => {
  runtime(); let templates: TaskTemplate[] = [];
  const confirm = vi.fn(async () => true); const alert = vi.fn(async () => {});
  const controller = createCommunityTemplateController({api: {} as EchosightApi, getTemplates: () => templates,
    saveTemplates: async next => {templates = next; return true;}, confirmUser: confirm, alertUser: alert, logger: {log() {},error(){}}});
  controller.loadCommunityTemplates();
  for (let i=0; i<6; i++) await controller.importCommunityTemplate(i);
  expect(templates.map(t=>t.name)).toEqual(COMMUNITY_TEMPLATES.map(t=>t.name));
  expect(templates.map(t=>t.tasks.length)).toEqual([8,9,8,8,8,8]);
  expect(document.getElementById('communityTemplatesModal')!.style.display).not.toBe('none');
  await controller.importCommunityTemplate(0); expect(confirm).toHaveBeenCalledWith('Template "League Start Essentials" already exists. Replace it?');
  controller.previewCommunityTemplate(0); expect(alert.mock.lastCall![0]).toContain('1. Reach Act 6 for resistance penalty');
});
