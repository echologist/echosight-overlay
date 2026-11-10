// Execute unchanged source workflow, completion feedback and recursive renderer.
import { afterEach, expect, test, vi } from 'vitest';
import type { EchosightApi, Task } from '../../src/shared/types';
import { createTaskStateController } from '../../src/renderer/features/tasks/taskStateController';
import { createBackgroundTaskController } from '../../src/renderer/features/tasks/backgroundTaskController';
import { createTaskWorkflowController } from '../../src/renderer/features/tasks/taskWorkflowController';
import { renderTaskLists } from '../../src/renderer/features/tasks/taskListView';

const task = (id: number, extra: Partial<Task> = {}): Task => ({
  id, text: `Task ${id}`, completed: false, createdAt: '2027-01-15T08:00:00.000Z',
  children: [], mode: 'main', triggers: [], activated: true, activatedAt: null,
  backgroundOptions: null, ...extra
});
const bg = (id: number, extra: Partial<Task> = {}) => task(id, {mode: 'background', activated: false, ...extra});
afterEach(() => { vi.restoreAllMocks(); vi.useRealTimers(); document.body.replaceChildren(); });

function runtime(tasks: Task[]) {
  vi.useFakeTimers(); vi.setSystemTime(1_800_000_000_000);
  vi.spyOn(Math, 'random').mockReturnValue(0.017);
  document.body.innerHTML = '<button id="complete">Complete</button><div id="progressText"></div><ul id="taskList"></ul><section id="backgroundTasksSection"><span id="backgroundCount"></span><ul id="backgroundTaskList"></ul></section>';
  const saveTasks = vi.fn(async () => ({success: true}));
  const state = createTaskStateController({api: {saveTasks} as unknown as EchosightApi, logger: {log() {}, error(e) {throw e;}}});
  state.replaceTasks(tasks, null);
  const render = () => renderTaskLists(state.getTasks(), {
    onToggleTask() {}, onContextMenu() {}, onConfigureTriggers() {}, onDeleteTask() {}
  });
  const background = createBackgroundTaskController({getTasks: state.getTasks, onChanged() {render(); void state.saveTasks();}, onActivated() {}, logger: {log() {}}});
  const sound = vi.fn();
  const workflow = createTaskWorkflowController({backgroundTasks: background, taskState: state,
    isInteractive: () => true, renderTasks: render, updateProgress() {}, saveTasks: state.saveTasks,
    playThemeSound: sound, logger: {log() {}, error(e) {throw e;}}});
  document.getElementById('complete')!.addEventListener('click', workflow.completeNextTask);
  render();
  return {state, saveTasks, sound, complete: () => document.getElementById('complete')!.click()};
}
const ids = (list: string) => Array.from(document.querySelectorAll(`#${list} [data-task-id]`), el => Number((el as HTMLElement).dataset.taskId));

test('complete feedback names actual main-root completion, not mixed-mode background descendant', () => {
  const r = runtime([bg(20, {activated: true, children: [task(21, {text: 'Skipped child'})]}), task(1, {text: 'Actual completion'})]);
  r.complete();
  expect(r.state.getTasks()[0].children[0].completed).toBe(false);
  expect(r.state.getTasks()[1].completed).toBe(true);
  expect(document.getElementById('progressText')!.textContent).toBe('✓ Completed: Actual completion');
  expect(r.saveTasks).toHaveBeenCalledTimes(1);
  expect(r.sound).toHaveBeenCalledExactlyOnceWith('taskCompleted');
});

test('no main root means no completion, feedback, save or sound', () => {
  const r = runtime([bg(20, {activated: true, children: [task(21, {text: 'Skipped child'})]})]);
  const before = JSON.stringify(r.state.getTasks());
  r.complete();
  expect(JSON.stringify(r.state.getTasks())).toBe(before);
  expect(document.getElementById('progressText')!.textContent).toBe('');
  expect(r.saveTasks).not.toHaveBeenCalled();
  expect(r.sound).not.toHaveBeenCalled();
});

test('trigger activates only parent but renderer includes dormant child', () => {
  const r = runtime([task(1, {triggers: [20]}), bg(20, {children: [bg(21)]})]);
  expect(ids('backgroundTaskList')).toEqual([]);
  r.complete();
  expect(r.state.getTasks()[1].activated).toBe(true);
  expect(r.state.getTasks()[1].children[0].activated).toBe(false);
  expect(ids('backgroundTaskList')).toEqual([20, 21]);
});

test('root-only mode and activation filtering preserves legacy mixed-mode descendants', () => {
  runtime([task(1, {children: [bg(21)]}), bg(20, {activated: true, children: [task(22)]}), bg(23)]);
  expect(ids('taskList')).toEqual([1, 21]);
  expect(ids('backgroundTaskList')).toEqual([20, 22]);
});
