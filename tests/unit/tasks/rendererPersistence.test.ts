import { beforeEach, describe, expect, test, vi } from 'vitest';
import type {
  EchosightApi,
  TaskLoadData
} from '../../../src/shared/types';

const mocks = vi.hoisted(() => ({
  alert: vi.fn(async (): Promise<void> => undefined),
  confirm: vi.fn(async () => false),
  showReorderFeedback: vi.fn()
}));

vi.mock('../../../src/renderer/ui/dialogService', () => ({
  createDialogService: () => ({
    alert: mocks.alert,
    confirm: mocks.confirm
  })
}));

vi.mock('../../../src/renderer/ui/feedbackUi', async importOriginal => ({
  ...await importOriginal<typeof import('../../../src/renderer/ui/feedbackUi')>(),
  showReorderFeedback: mocks.showReorderFeedback
}));

describe('renderer task persistence funnel', () => {
  beforeEach(() => {
    vi.resetModules();
    vi.clearAllMocks();
    document.body.innerHTML = '';
  });

  test('shows one startup alert naming quarantined task backup', async () => {
    const corruptBackupPath = '/data/tasks.json.corrupt-2026';
    let resolveAlert: () => void = () => undefined;
    mocks.alert.mockImplementationOnce(() => new Promise<void>(resolve => {
      resolveAlert = resolve;
    }));
    const api = createApi({
      tasks: [],
      currentTemplate: null,
      snapshots: [],
      corruptBackupPath
    });
    api.saveTasks = vi.fn().mockResolvedValue({ success: true });
    setApi(api);
    const renderer = await import('../../../src/renderer/renderer');

    const load = renderer.loadTasks();
    const completed = vi.fn();
    void load.then(completed);

    try {
      await vi.waitFor(() => expect(mocks.alert).toHaveBeenCalledOnce());
      await vi.waitFor(() => expect(completed).toHaveBeenCalledOnce(), { timeout: 100 });
      await expect(renderer.saveTasks()).resolves.toBe(true);

      expect(mocks.alert).toHaveBeenCalledWith(
        expect.stringContaining(corruptBackupPath),
        expect.objectContaining({ title: 'Task Data Recovered' })
      );
      expect(api.saveTasks).toHaveBeenCalledOnce();
    } finally {
      resolveAlert();
      await load;
    }
  });

  test('shows visible feedback when task save fails', async () => {
    const api = createApi({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
    api.saveTasks = vi.fn().mockResolvedValue({
      success: false,
      error: 'disk full'
    });
    setApi(api);
    const renderer = await import('../../../src/renderer/renderer');

    await renderer.loadTasks();
    await expect(renderer.saveTasks()).resolves.toBe(false);
    expect(mocks.showReorderFeedback).toHaveBeenCalledWith(
      'Failed to save tasks. Changes may not persist.'
    );
  });

  test('rejected task load leaves saving blocked without calling the API', async () => {
    const api = createApi({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
    api.loadTasks = vi.fn().mockRejectedValue(new Error('load failed'));
    api.saveTasks = vi.fn().mockResolvedValue({ success: true });
    setApi(api);
    const renderer = await import('../../../src/renderer/renderer');

    await expect(renderer.loadTasks()).rejects.toThrow('load failed');
    await expect(renderer.saveTasks()).resolves.toBe(false);

    expect(api.saveTasks).not.toHaveBeenCalled();
    expect(mocks.showReorderFeedback).toHaveBeenCalledWith(
      'Cannot save tasks until task data loads successfully.'
    );
  });

  test('task load failure is reported without aborting initialization or enabling saves', async () => {
    const api = createApi({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
    api.loadTasks = vi.fn().mockRejectedValue(new Error('permission denied'));
    api.saveTasks = vi.fn().mockResolvedValue({ success: true });
    setApi(api);
    const renderer = await import('../../../src/renderer/renderer');

    await expect(renderer.loadTasksForInitialization()).resolves.toBeUndefined();
    await expect(renderer.saveTasks()).resolves.toBe(false);

    expect(api.saveTasks).not.toHaveBeenCalled();
    expect(mocks.alert).toHaveBeenCalledWith(
      expect.stringContaining('permission denied'),
      expect.objectContaining({ title: 'Task Data Unavailable', tone: 'danger' })
    );
  });

  test('task reload disables saving while load remains pending', async () => {
    const api = createApi({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
    api.saveTasks = vi.fn().mockResolvedValue({ success: true });
    setApi(api);
    const renderer = await import('../../../src/renderer/renderer');
    await renderer.loadTasks();

    api.loadTasks = vi.fn(() => new Promise<TaskLoadData>(() => undefined));
    void renderer.loadTasks();
    await expect(renderer.saveTasks()).resolves.toBe(false);

    expect(api.saveTasks).not.toHaveBeenCalled();
  });
});

function setApi(api: EchosightApi): void {
  Object.defineProperty(window, 'echosight', {
    configurable: true,
    value: api
  });
}

function createApi(taskState: TaskLoadData): EchosightApi {
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
