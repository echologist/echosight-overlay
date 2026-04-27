import { describe, expect, test } from 'vitest';
import type {
  EchosightApi,
  Task,
  TaskSaveData
} from '../../../src/shared/types';
import { createTaskStateController } from '../../../src/renderer/features/tasks/taskStateController';

describe('task state history', () => {
  test('undo and forward restore task completion state', async () => {
    const api = createTaskApi({
      tasks: [createTask({ id: 1, text: 'Open map' })],
      currentTemplate: null
    });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await controller.loadTasks();
    expect(controller.getTasks()[0].completed).toBe(false);

    controller.toggleTask(1);
    expect(controller.getTasks()[0].completed).toBe(true);

    expect(controller.undoLastAction()).toEqual({
      restored: true,
      label: 'toggle task'
    });
    expect(controller.getTasks()[0].completed).toBe(false);

    expect(controller.redoLastAction()).toEqual({
      restored: true,
      label: 'toggle task'
    });
    expect(controller.getTasks()[0].completed).toBe(true);
  });

  test('new mutations clear the forward stack', async () => {
    const api = createTaskApi({
      tasks: [createTask({ id: 1, text: 'Open map' })],
      currentTemplate: null
    });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await controller.loadTasks();
    controller.toggleTask(1);
    controller.undoLastAction();

    controller.addTask('Roll map');

    expect(controller.redoLastAction()).toEqual({ restored: false });
    expect(controller.getTasks().map(task => task.text)).toEqual(['Open map', 'Roll map']);
  });

  test('delete undo and forward preserve cleaned trigger references', async () => {
    const backgroundTask = createTask({
      id: 20,
      text: 'Check altar mods',
      mode: 'background',
      activated: false
    });
    const api = createTaskApi({
      tasks: [
        createTask({
          id: 10,
          text: 'Kill boss',
          triggers: [backgroundTask.id]
        }),
        backgroundTask
      ],
      currentTemplate: null
    });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await controller.loadTasks();
    expect(controller.deleteTask(backgroundTask.id).removed).toBe(true);
    expect(controller.getTasks().map(task => task.id)).toEqual([10]);
    expect(controller.getTasks()[0].triggers).toEqual([]);

    expect(controller.undoLastAction().restored).toBe(true);
    expect(controller.getTasks().map(task => task.id)).toEqual([10, 20]);
    expect(controller.getTasks()[0].triggers).toEqual([20]);

    expect(controller.redoLastAction().restored).toBe(true);
    expect(controller.getTasks().map(task => task.id)).toEqual([10]);
    expect(controller.getTasks()[0].triggers).toEqual([]);
  });

  test('saveTasks persists only the last five auto-save snapshots', async () => {
    const api = createTaskApi({
      tasks: [createTask({ id: 1, text: 'Snapshot target' })],
      currentTemplate: 'Mapping'
    });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await controller.loadTasks();
    for (let index = 0; index < 6; index++) {
      await controller.saveTasks();
    }

    const latestSave = api.saved.at(-1);
    expect(latestSave?.currentTemplate).toBe('Mapping');
    expect(latestSave?.snapshots).toHaveLength(5);
    expect(latestSave?.snapshots?.at(-1)?.state.tasks[0].text).toBe('Snapshot target');
  });
});

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

function createTaskApi(initialState: TaskSaveData): EchosightApi & { saved: TaskSaveData[] } {
  const saved: TaskSaveData[] = [];

  return {
    saved,
    loadTasks: async () => clone(initialState),
    saveTasks: async (taskState) => {
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

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T;
}
