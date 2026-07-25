import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  beforeEach,
  describe,
  expect,
  test,
  vi
} from 'vitest';
import type {
  EchosightApi,
  Task,
  TaskLoadData,
  TaskSaveData
} from '../../../src/shared/types';

describe('renderer task editing', () => {
  beforeEach(() => {
    vi.resetModules();
    document.documentElement.innerHTML = readRendererHtml();
  });

  test('edits nested and active background tasks through the right-click workflow', async () => {
    const backgroundTask = createTask({
      id: 20,
      text: 'Watch timer',
      completed: true,
      createdAt: '2026-07-25T02:00:00.000Z',
      mode: 'background',
      triggers: [77],
      activated: true,
      activatedAt: '2026-07-25T03:00:00.000Z',
      backgroundOptions: {
        expiresAfterMinutes: 15,
        priority: 'normal'
      }
    });
    const child = createTask({
      id: 2,
      text: 'Nested objective',
      completed: true,
      createdAt: '2026-07-25T01:00:00.000Z',
      triggers: [backgroundTask.id]
    });
    const dormantBackgroundTask = createTask({
      id: 30,
      text: 'Dormant timer',
      mode: 'background',
      activated: false,
      backgroundOptions: {
        expiresAfterMinutes: null,
        priority: 'normal'
      }
    });
    const initialState: TaskLoadData = {
      tasks: [
        createTask({
          id: 1,
          text: 'Parent objective',
          children: [child]
        }),
        backgroundTask,
        dormantBackgroundTask
      ],
      currentTemplate: null,
      snapshots: []
    };
    const saved: TaskSaveData[] = [];
    const api = createApi(initialState, saved);
    setApi(api);

    await import('../../../src/renderer/renderer');
    document.dispatchEvent(new Event('DOMContentLoaded'));
    await vi.waitFor(() => {
      expect(getTaskText('taskList', 2).textContent).toBe('Nested objective');
      expect(getTaskText('backgroundTaskList', 20).textContent).toBe('Watch timer');
    });

    openEditMenu('taskList', 2);
    getInput('taskEditInput').value = 'Cancelled change';
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));

    expect(getElement('taskEditModal').classList.contains('is-visible')).toBe(false);
    expect(getTaskText('taskList', 2).textContent).toBe('Nested objective');
    expect(saved).toHaveLength(0);

    openEditMenu('taskList', 2);
    getInput('taskEditInput').value = '  Renamed objective  ';
    getButton('saveTaskEditButton').click();

    await vi.waitFor(() => {
      expect(getTaskText('taskList', 2).textContent).toBe('Renamed objective');
      expect(saved).toHaveLength(1);
    });

    const savedChild = saved[0].tasks[0].children[0];
    expect(savedChild).toMatchObject({
      id: 2,
      text: 'Renamed objective',
      completed: true,
      createdAt: '2026-07-25T01:00:00.000Z',
      triggers: [20]
    });
    expect(saved[0].tasks[0].children).toHaveLength(1);

    openEditMenu('backgroundTaskList', 20);
    expect(getElement('taskEditBackgroundOptions').hidden).toBe(false);
    expect(getInput('taskEditInput').value).toBe('Watch timer');
    expect(getInput('taskEditHighPriority').checked).toBe(false);

    getInput('taskEditInput').value = '  Watch new timer  ';
    getInput('taskEditHighPriority').checked = true;
    getButton('saveTaskEditButton').click();

    await vi.waitFor(() => {
      expect(getTaskText('backgroundTaskList', 20).textContent).toBe('Watch new timer');
      expect(getTaskItem('backgroundTaskList', 20).classList.contains('high-priority')).toBe(true);
      expect(saved).toHaveLength(2);
    });

    const finalMainTask = saved[1].tasks[0];
    const finalBackgroundTask = saved[1].tasks[1];
    expect(finalMainTask.children[0].triggers).toEqual([20]);
    expect(finalBackgroundTask).toEqual({
      ...backgroundTask,
      text: 'Watch new timer',
      backgroundOptions: {
        expiresAfterMinutes: 15,
        priority: 'high'
      }
    });

    clickContextMenuItem('taskList', 2, 'Configure Triggers');
    const dormantCheckbox = document.querySelector<HTMLInputElement>(
      '.trigger-checkbox[data-bg-task-id="30"]'
    );
    const dormantEditButton = dormantCheckbox
      ?.closest('.trigger-task-row')
      ?.querySelector<HTMLButtonElement>('.trigger-task-edit');
    expect(dormantCheckbox?.checked).toBe(false);
    dormantCheckbox?.click();
    dormantEditButton?.click();

    const triggerModal = getElement('configureTriggersModal');
    const editModal = getElement('taskEditModal');
    expect(triggerModal.classList.contains('is-visible')).toBe(true);
    expect(editModal.classList.contains('is-visible')).toBe(true);
    expect(
      triggerModal.compareDocumentPosition(editModal) & Node.DOCUMENT_POSITION_FOLLOWING
    ).not.toBe(0);
    expect(getInput('taskEditInput').value).toBe('Dormant timer');

    getInput('taskEditInput').value = '  Renamed dormant timer  ';
    getInput('taskEditHighPriority').checked = true;
    getButton('saveTaskEditButton').click();

    await vi.waitFor(() => expect(saved).toHaveLength(3));
    expect(saved[2].tasks[2]).toEqual({
      ...dormantBackgroundTask,
      text: 'Renamed dormant timer',
      backgroundOptions: {
        expiresAfterMinutes: null,
        priority: 'high'
      }
    });
    expect(dormantCheckbox?.checked).toBe(true);

    getButton('saveTriggersButton').click();
    await vi.waitFor(() => expect(saved).toHaveLength(4));
    expect(saved[3].tasks[0].children[0].triggers).toEqual([20, 30]);
  });
});

function openEditMenu(listId: string, taskId: number): void {
  clickContextMenuItem(listId, taskId, 'Edit task');
}

function clickContextMenuItem(listId: string, taskId: number, label: string): void {
  getTaskText(listId, taskId).dispatchEvent(new MouseEvent('contextmenu', {
    bubbles: true,
    clientX: 20,
    clientY: 30
  }));
  const item = Array.from(document.querySelectorAll<HTMLElement>('.context-menu div'))
    .find(menuItem => menuItem.textContent === label);
  if (!item) {
    throw new Error(`Missing ${label} menu item`);
  }
  item.click();
}

function getTaskItem(listId: string, taskId: number): HTMLElement {
  const item = document.querySelector<HTMLElement>(`#${listId} [data-task-id="${taskId}"]`);
  if (!item) {
    throw new Error(`Missing task ${taskId} in ${listId}`);
  }
  return item;
}

function getTaskText(listId: string, taskId: number): HTMLElement {
  const text = getTaskItem(listId, taskId).querySelector<HTMLElement>('.task-text');
  if (!text) {
    throw new Error(`Missing task text ${taskId} in ${listId}`);
  }
  return text;
}

function getElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing element ${id}`);
  }
  return element;
}

function getInput(id: string): HTMLInputElement {
  const element = getElement(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`Element ${id} is not an input`);
  }
  return element;
}

function getButton(id: string): HTMLButtonElement {
  const element = getElement(id);
  if (!(element instanceof HTMLButtonElement)) {
    throw new Error(`Element ${id} is not a button`);
  }
  return element;
}

function readRendererHtml(): string {
  return readFileSync(join(process.cwd(), 'src/renderer/index.html'), 'utf8');
}

function setApi(api: EchosightApi): void {
  Object.defineProperty(window, 'echosight', {
    configurable: true,
    value: api
  });
}

function createApi(initialState: TaskLoadData, saved: TaskSaveData[]): EchosightApi {
  return {
    loadTasks: async () => clone(initialState),
    saveTasks: async taskState => {
      saved.push(clone(taskState));
      return { success: true };
    },
    loadTemplates: async () => [],
    saveTemplates: async () => ({ success: true }),
    loadSettings: async () => null,
    saveSettings: async () => ({ success: true }),
    loadThemes: async () => [],
    getTheme: async () => null,
    reloadThemes: async () => [],
    openThemesFolder: async () => ({ success: true }),
    getThemesPath: async () => '',
    loadThemeCss: async () => null,
    getThemeAsset: async () => null,
    updateHotkeys: () => undefined,
    setHotkeyRecording: () => undefined,
    focusWindow: () => undefined,
    resetWindowPosition: () => undefined,
    toggleOverlay: () => undefined,
    minimizeOverlay: () => undefined,
    toggleInteractiveMode: () => undefined,
    quitApplication: () => undefined,
    onInteractiveModeChanged: () => () => undefined,
    onCompleteNextTask: () => () => undefined,
    onUndoLastTaskAction: () => () => undefined,
    onRedoLastTaskAction: () => () => undefined
  };
}

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

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
