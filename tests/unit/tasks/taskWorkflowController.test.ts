import { describe, expect, test, vi } from 'vitest';
import type {
  EchosightApi,
  Task,
  TaskSaveData
} from '../../../src/shared/types';
import type { BackgroundTaskController } from '../../../src/renderer/features/tasks/backgroundTaskController';
import { createTaskStateController } from '../../../src/renderer/features/tasks/taskStateController';
import { createTaskWorkflowController } from '../../../src/renderer/features/tasks/taskWorkflowController';

describe('task workflow controller', () => {
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
  const confirmMessages: string[] = [];
  const renderTasks = vi.fn();
  const saveTasks = vi.fn();
  const updateProgress = vi.fn();
  const playThemeSound = vi.fn();

  const workflow = createTaskWorkflowController({
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
