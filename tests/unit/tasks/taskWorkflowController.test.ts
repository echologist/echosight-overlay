import { afterEach, describe, expect, test, vi } from 'vitest';
import type {
  EchosightApi,
  Task,
  TaskSaveData
} from '../../../src/shared/types';
import type { BackgroundTaskController } from '../../../src/renderer/features/tasks/backgroundTaskController';
import { createTaskStateController } from '../../../src/renderer/features/tasks/taskStateController';
import { createTaskWorkflowController } from '../../../src/renderer/features/tasks/taskWorkflowController';

describe('task workflow controller', () => {
  afterEach(() => {
    vi.useRealTimers();
    document.body.replaceChildren();
  });

  test('edits a nested normal task through the modal and persists once', async () => {
    vi.useFakeTimers();
    installTaskEditFixture();
    const harness = await createWorkflowHarness({
      initialState: {
        tasks: [
          createTask({
            id: 1,
            text: 'Parent',
            children: [createTask({ id: 2, text: 'Old child' })]
          })
        ],
        currentTemplate: null
      },
      confirmResult: false
    });

    harness.workflow.openTaskEditor(2);
    vi.advanceTimersByTime(50);

    const input = getInput('taskEditInput');
    expect(input.value).toBe('Old child');
    expect(input.selectionStart).toBe(0);
    expect(input.selectionEnd).toBe('Old child'.length);
    expect(getElement('taskEditBackgroundOptions').hidden).toBe(true);

    input.value = '  Renamed child  ';
    harness.workflow.saveTaskEdit();

    expect(harness.taskState.getTasks()[0].children[0].text).toBe('Renamed child');
    expect(harness.renderTasks).toHaveBeenCalledOnce();
    expect(harness.updateProgress).toHaveBeenCalledOnce();
    expect(harness.saveTasks).toHaveBeenCalledOnce();
    expect(getElement('taskEditModal').classList.contains('is-visible')).toBe(false);
  });

  test('edits background text and priority while preserving activation and expiration', async () => {
    installTaskEditFixture();
    const backgroundTask = createTask({
      id: 20,
      text: 'Old background',
      mode: 'background',
      activated: true,
      activatedAt: '2026-04-27T02:00:00.000Z',
      backgroundOptions: {
        expiresAfterMinutes: 15,
        priority: 'normal'
      }
    });
    const harness = await createWorkflowHarness({
      initialState: {
        tasks: [backgroundTask],
        currentTemplate: null
      },
      confirmResult: false
    });

    harness.workflow.openTaskEditor(20);

    expect(getElement('taskEditBackgroundOptions').hidden).toBe(false);
    expect(getInput('taskEditHighPriority').checked).toBe(false);

    getInput('taskEditInput').value = '  New background  ';
    getInput('taskEditHighPriority').checked = true;
    harness.workflow.saveTaskEdit();

    expect(harness.taskState.getTasks()[0]).toMatchObject({
      text: 'New background',
      activated: true,
      activatedAt: '2026-04-27T02:00:00.000Z',
      backgroundOptions: {
        expiresAfterMinutes: 15,
        priority: 'high'
      }
    });
    expect(harness.renderTasks).toHaveBeenCalledOnce();
    expect(harness.updateProgress).toHaveBeenCalledOnce();
    expect(harness.saveTasks).toHaveBeenCalledOnce();
  });

  test('rejects empty edit text without closing, mutating, or persisting', async () => {
    installTaskEditFixture();
    const harness = await createWorkflowHarness({
      initialState: {
        tasks: [createTask({ id: 1, text: 'Keep me' })],
        currentTemplate: null
      },
      confirmResult: false
    });
    const previousState = clone(harness.taskState.getTasks());

    harness.workflow.openTaskEditor(1);
    getInput('taskEditInput').value = '   ';
    harness.workflow.saveTaskEdit();

    expect(harness.alertMessages).toEqual(['Please enter a task name!']);
    expect(getElement('taskEditModal').classList.contains('is-visible')).toBe(true);
    expect(harness.taskState.getTasks()).toEqual(previousState);
    expect(harness.taskState.undoLastAction()).toEqual({ restored: false });
    expect(harness.renderTasks).not.toHaveBeenCalled();
    expect(harness.updateProgress).not.toHaveBeenCalled();
    expect(harness.saveTasks).not.toHaveBeenCalled();
  });

  test('cancels changed edit controls without mutating or persisting', async () => {
    installTaskEditFixture();
    const harness = await createWorkflowHarness({
      initialState: {
        tasks: [createTask({ id: 1, text: 'Keep me' })],
        currentTemplate: null
      },
      confirmResult: false
    });
    const previousState = clone(harness.taskState.getTasks());

    harness.workflow.openTaskEditor(1);
    getInput('taskEditInput').value = 'Changed';
    getInput('taskEditHighPriority').checked = true;
    harness.workflow.closeTaskEditor();

    expect(getElement('taskEditModal').classList.contains('is-visible')).toBe(false);
    expect(harness.taskState.getTasks()).toEqual(previousState);
    expect(harness.taskState.undoLastAction()).toEqual({ restored: false });
    expect(harness.renderTasks).not.toHaveBeenCalled();
    expect(harness.saveTasks).not.toHaveBeenCalled();
  });

  test('closes a normalized no-op edit without history or persistence', async () => {
    installTaskEditFixture();
    const harness = await createWorkflowHarness({
      initialState: {
        tasks: [createTask({ id: 1, text: 'Keep me' })],
        currentTemplate: null
      },
      confirmResult: false
    });

    harness.workflow.openTaskEditor(1);
    getInput('taskEditInput').value = '  Keep me  ';
    harness.workflow.saveTaskEdit();

    expect(getElement('taskEditModal').classList.contains('is-visible')).toBe(false);
    expect(harness.taskState.undoLastAction()).toEqual({ restored: false });
    expect(harness.renderTasks).not.toHaveBeenCalled();
    expect(harness.updateProgress).not.toHaveBeenCalled();
    expect(harness.saveTasks).not.toHaveBeenCalled();
  });

  test('delete cancellation leaves task state untouched', async () => {
    const harness = await createWorkflowHarness({
      initialState: {
        tasks: [createTask({ id: 1, text: 'Keep me' })],
        currentTemplate: null
      },
      confirmResult: false
    });

    await harness.workflow.deleteTask(1);

    expect(harness.confirmMessages).toEqual([
      'Delete "Keep me"? You can undo this with Ctrl+Shift+Z.'
    ]);
    expect(harness.taskState.getTasks()).toHaveLength(1);
    expect(harness.saveTasks).not.toHaveBeenCalled();
    expect(harness.renderTasks).not.toHaveBeenCalled();
  });

  test('confirmed clear all tasks persists and clears timers', async () => {
    const harness = await createWorkflowHarness({
      initialState: {
        tasks: [
          createTask({ id: 1, text: 'One' }),
          createTask({ id: 2, text: 'Two' })
        ],
        currentTemplate: null
      },
      confirmResult: true
    });

    await harness.workflow.clearAllTasks();

    expect(harness.confirmMessages).toEqual([
      'Clear all 2 tasks? You can undo this with Ctrl+Shift+Z.'
    ]);
    expect(harness.taskState.getTasks()).toEqual([]);
    expect(harness.backgroundTasks.clearAllExpirationTimers).toHaveBeenCalledOnce();
    expect(harness.saveTasks).toHaveBeenCalledOnce();
    expect(harness.renderTasks).toHaveBeenCalledOnce();
    expect(harness.updateProgress).toHaveBeenCalledOnce();
  });

  test('undo and forward restart background timers and persist restored state', async () => {
    const harness = await createWorkflowHarness({
      initialState: {
        tasks: [createTask({ id: 1, text: 'Toggle me' })],
        currentTemplate: null
      },
      confirmResult: true
    });

    harness.workflow.toggleTask(1);
    expect(harness.taskState.getTasks()[0].completed).toBe(true);

    harness.workflow.undoLastAction();
    expect(harness.taskState.getTasks()[0].completed).toBe(false);

    harness.workflow.redoLastAction();
    expect(harness.taskState.getTasks()[0].completed).toBe(true);

    expect(harness.backgroundTasks.clearAllExpirationTimers).toHaveBeenCalledTimes(2);
    expect(harness.backgroundTasks.restartExpirationTimers).toHaveBeenCalledTimes(2);
    expect(harness.saveTasks).toHaveBeenCalledTimes(3);
    expect(harness.playThemeSound).toHaveBeenCalledWith('taskCompleted');
    expect(harness.playThemeSound).toHaveBeenCalledWith('undo');
    expect(harness.playThemeSound).toHaveBeenCalledWith('redo');
  });
});

interface WorkflowHarnessOptions {
  confirmResult: boolean;
  initialState: TaskSaveData;
}

async function createWorkflowHarness(options: WorkflowHarnessOptions) {
  const api = createTaskApi(options.initialState);
  const taskState = createTaskStateController({ api, logger: silentLogger });
  await taskState.loadTasks();

  const backgroundTasks: BackgroundTaskController = {
    activateTriggeredTasks: vi.fn(),
    deactivateTriggeredTasks: vi.fn(),
    restartExpirationTimers: vi.fn(),
    clearExpirationTimer: vi.fn(),
    clearAllExpirationTimers: vi.fn()
  };
  const alertMessages: string[] = [];
  const confirmMessages: string[] = [];
  const renderTasks = vi.fn();
  const saveTasks = vi.fn();
  const updateProgress = vi.fn();
  const playThemeSound = vi.fn();

  const workflow = createTaskWorkflowController({
    alertUser: message => {
      alertMessages.push(message);
    },
    api: {
      focusWindow: vi.fn()
    },
    backgroundTasks,
    confirmUser: message => {
      confirmMessages.push(message);
      return options.confirmResult;
    },
    isInteractive: () => false,
    logger: silentLogger,
    playThemeSound,
    renderTasks,
    saveTasks,
    taskState,
    updateProgress
  });

  return {
    alertMessages,
    backgroundTasks,
    confirmMessages,
    renderTasks,
    saveTasks,
    taskState,
    updateProgress,
    playThemeSound,
    workflow
  };
}

function installTaskEditFixture(): void {
  document.body.innerHTML = `
    <div class="modal" id="taskEditModal" aria-hidden="true">
      <div class="modal-content">
        <input id="taskEditInput">
        <div id="taskEditBackgroundOptions" hidden>
          <input type="checkbox" id="taskEditHighPriority">
        </div>
      </div>
    </div>
  `;
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

const silentLogger = {
  log: () => undefined,
  error: () => undefined
};

function createTask(overrides: Partial<Task> & Pick<Task, 'id' | 'text'>): Task {
  const { id, text, ...rest } = overrides;

  return {
    id,
    text,
    completed: false,
    createdAt: '2026-04-27T00:00:00.000Z',
    children: [],
    mode: 'main',
    triggers: [],
    activated: true,
    activatedAt: null,
    backgroundOptions: null,
    ...rest
  };
}

function createTaskApi(initialState: TaskSaveData): EchosightApi {
  return {
    loadTasks: async () => clone(initialState),
    saveTasks: async () => ({ success: true }),
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

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
