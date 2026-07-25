import {
  afterEach,
  describe,
  expect,
  test,
  vi
} from 'vitest';
import type { Task } from '../../../src/shared/types';
import { createTriggerController } from '../../../src/renderer/features/tasks/triggerController';

describe('trigger controller background task editing', () => {
  afterEach(() => {
    vi.useRealTimers();
    document.body.replaceChildren();
  });

  test('opens the shared editor for a dormant background row without changing trigger selection', () => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <div id="configureTriggersModal" class="modal">
        <div id="triggerTaskList"></div>
        <input id="newBgTaskInput">
        <input id="bgTaskHighPriority" type="checkbox">
      </div>
    `;
    const dormantBackground = createTask({
      id: 20,
      text: 'Dormant timer',
      mode: 'background',
      activated: false
    });
    const mainTask = createTask({
      id: 1,
      text: 'Main objective',
      triggers: [dormantBackground.id]
    });
    const tasks = [mainTask, dormantBackground];
    const openTaskEditor = vi.fn();
    const onChanged = vi.fn();
    const controller = createTriggerController({
      addBackgroundTask: vi.fn(),
      findTaskById: id => tasks.find(task => task.id === id) ?? null,
      getTasks: () => tasks,
      onChanged,
      openTaskEditor
    });

    controller.configureTriggers(mainTask.id);

    const checkbox = document.querySelector<HTMLInputElement>('.trigger-checkbox');
    const editButton = document.querySelector<HTMLButtonElement>('.trigger-task-edit');
    expect(checkbox?.checked).toBe(true);
    expect(editButton?.textContent).toBe('Edit');

    editButton?.click();

    expect(openTaskEditor).toHaveBeenCalledWith(dormantBackground.id);
    expect(checkbox?.checked).toBe(true);

    if (checkbox) {
      checkbox.checked = false;
    }
    controller.saveTriggers();
    expect(mainTask.triggers).toEqual([]);
    expect(onChanged).toHaveBeenCalledOnce();
  });
});

function createTask(overrides: Partial<Task> & Pick<Task, 'id' | 'text'>): Task {
  const { id, text, ...rest } = overrides;
  return {
    id,
    text,
    completed: false,
    createdAt: '2026-07-25T00:00:00.000Z',
    children: [],
    mode: 'main',
    triggers: [],
    activated: true,
    activatedAt: null,
    backgroundOptions: null,
    ...rest
  };
}
