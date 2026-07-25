import { describe, expect, test } from 'vitest';
import type {
  EchosightApi,
  Task,
  TaskLoadData,
  TaskSaveData
} from '../../../src/shared/types';
import { createTaskStateController } from '../../../src/renderer/features/tasks/taskStateController';

describe('task state history', () => {
  test('edits a nested task in place without changing its state or hierarchy', async () => {
    const child = createTask({
      id: 2,
      text: 'Old child',
      completed: true,
      createdAt: '2026-04-27T01:00:00.000Z',
      triggers: [20],
      activated: false,
      activatedAt: '2026-04-27T02:00:00.000Z',
      backgroundOptions: {
        expiresAfterMinutes: 10,
        priority: 'normal'
      }
    });
    const api = createTaskApi({
      tasks: [createTask({ id: 1, text: 'Parent', children: [child] })],
      currentTemplate: null
    });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await controller.loadTasks();
    const nestedTask = controller.getTasks()[0].children[0];
    const previousState = clone(nestedTask);

    expect(controller.editTask(2, '  Renamed child  ', 'high')).toBe(true);
    expect(controller.getTasks()[0].children[0]).toBe(nestedTask);
    expect(nestedTask).toEqual({
      ...previousState,
      text: 'Renamed child'
    });
  });

  test('edits and restores background text and priority without changing other fields', async () => {
    const backgroundTask = createTask({
      id: 20,
      text: 'Old background',
      completed: true,
      createdAt: '2026-04-27T01:00:00.000Z',
      children: [createTask({ id: 21, text: 'Background child' })],
      mode: 'background',
      triggers: [30],
      activated: true,
      activatedAt: '2026-04-27T02:00:00.000Z',
      backgroundOptions: {
        expiresAfterMinutes: 15,
        priority: 'normal'
      }
    });
    const api = createTaskApi({
      tasks: [backgroundTask],
      currentTemplate: null
    });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await controller.loadTasks();
    const previousState = clone(controller.getTasks()[0]);

    expect(controller.editTask(20, '  New background  ', 'high')).toBe(true);
    expect(controller.getTasks()[0]).toEqual({
      ...previousState,
      text: 'New background',
      backgroundOptions: {
        expiresAfterMinutes: 15,
        priority: 'high'
      }
    });

    expect(controller.undoLastAction()).toEqual({
      restored: true,
      label: 'edit task'
    });
    expect(controller.getTasks()[0]).toEqual(previousState);

    expect(controller.redoLastAction()).toEqual({
      restored: true,
      label: 'edit task'
    });
    expect(controller.getTasks()[0]).toEqual({
      ...previousState,
      text: 'New background',
      backgroundOptions: {
        expiresAfterMinutes: 15,
        priority: 'high'
      }
    });
  });

  test('rejects empty, missing, and normalized no-op edits without history', async () => {
    const api = createTaskApi({
      tasks: [createTask({ id: 1, text: 'Keep me' })],
      currentTemplate: null
    });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await controller.loadTasks();
    const previousState = clone(controller.getTasks());

    expect(controller.editTask(1, '   ')).toBe(false);
    expect(controller.editTask(999, 'Missing')).toBe(false);
    expect(controller.editTask(1, '  Keep me  ', 'high')).toBe(false);
    expect(controller.getTasks()).toEqual(previousState);
    expect(controller.undoLastAction()).toEqual({ restored: false });
  });

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

  test('returns corrupt backup path from loadTasks', async () => {
    const api = createTaskApi({
      tasks: [],
      currentTemplate: null,
      snapshots: [],
      corruptBackupPath: '/data/tasks.json.corrupt-2026'
    });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await expect(controller.loadTasks()).resolves.toMatchObject({
      corruptBackupPath: '/data/tasks.json.corrupt-2026'
    });
  });

  test('returns false when persistence reports save failure', async () => {
    const api = createTaskApi({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
    api.saveTasks = async () => ({ success: false, error: 'disk full' });
    const controller = createTaskStateController({ api, logger: silentLogger });

    await controller.loadTasks();

    await expect(controller.saveTasks()).resolves.toBe(false);
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

function createTaskApi(initialState: TaskLoadData): EchosightApi & { saved: TaskSaveData[] } {
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
