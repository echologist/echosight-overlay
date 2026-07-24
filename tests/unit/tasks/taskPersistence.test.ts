import { describe, expect, test, vi } from 'vitest';
import type {
  EchosightApi,
  TaskLoadData,
  TaskSaveData
} from '../../../src/shared/types';
import {
  loadTaskState,
  saveTaskState
} from '../../../src/renderer/features/tasks/taskPersistence';

describe('task persistence', () => {
  test('logs and rethrows rejected task loads', async () => {
    const failure = new Error('IPC unavailable');
    const api = createTaskApi();
    api.loadTasks = vi.fn().mockRejectedValue(failure);
    const logger = { error: vi.fn() };

    await expect(loadTaskState(api, logger)).rejects.toBe(failure);
    expect(logger.error).toHaveBeenCalledWith('Failed to load tasks:', failure);
  });

  test('preserves corrupt backup path while normalizing loaded state', async () => {
    const api = createTaskApi({
      tasks: [],
      currentTemplate: null,
      snapshots: [],
      corruptBackupPath: '/data/tasks.json.corrupt-2026'
    });

    await expect(loadTaskState(api, silentLogger)).resolves.toEqual({
      tasks: [],
      currentTemplate: null,
      snapshots: [],
      corruptBackupPath: '/data/tasks.json.corrupt-2026'
    });
  });

  test('returns false when task save reports failure', async () => {
    const api = createTaskApi();
    api.saveTasks = vi.fn().mockResolvedValue({
      success: false,
      error: 'disk full'
    });

    await expect(saveTaskState(api, emptyTaskState, silentLogger)).resolves.toBe(false);
  });
});

function createTaskApi(taskState: TaskLoadData = emptyTaskState): EchosightApi {
  return {
    loadTasks: async () => taskState,
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

const emptyTaskState: TaskSaveData = {
  tasks: [],
  currentTemplate: null,
  snapshots: []
};

const silentLogger = {
  error: () => undefined
};
