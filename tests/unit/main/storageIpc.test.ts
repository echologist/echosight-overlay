import { promises as fs } from 'fs';
import os from 'os';
import path from 'path';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { registerStorageIpc } from '../../../src/main/ipc/storageIpc';

describe('storage IPC loads', () => {
  let tempRoot = '';

  beforeEach(async () => {
    tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'echosight-storage-ipc-'));
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-07-24T12:34:56.789Z'));
  });

  afterEach(async () => {
    vi.useRealTimers();
    vi.restoreAllMocks();
    await fs.rm(tempRoot, { recursive: true, force: true });
  });

  test('missing task file returns empty state without quarantine', async () => {
    const harness = createHarness(createPaths(tempRoot));

    await expect(harness.invoke('load-tasks')).resolves.toEqual({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
    await expect(fs.readdir(tempRoot)).resolves.toEqual([]);
  });

  test('rejects EACCES without backing up or replacing the live file and blocks saves', async () => {
    const paths = createPaths(tempRoot);
    const bytes = Buffer.from('{"tasks":[]}', 'utf8');
    await fs.writeFile(paths.tasksFile, bytes);
    const accessError = Object.assign(new Error('permission denied'), { code: 'EACCES' });
    vi.spyOn(fs, 'readFile').mockRejectedValueOnce(accessError);
    const copySpy = vi.spyOn(fs, 'copyFile');
    const renameSpy = vi.spyOn(fs, 'rename');
    const harness = createHarness(paths);

    await expect(harness.invoke('load-tasks')).rejects.toBe(accessError);
    expect(copySpy).not.toHaveBeenCalled();
    expect(renameSpy).not.toHaveBeenCalled();
    await expect(fs.readFile(paths.tasksFile)).resolves.toEqual(bytes);
    await expect(harness.invoke('save-tasks', {
      tasks: [{ text: 'replacement' }],
      currentTemplate: null,
      snapshots: []
    })).resolves.toEqual({
      success: false,
      error: expect.stringContaining('permission denied')
    });
  });

  test.each([
    ['zero-byte', Buffer.alloc(0)],
    ['truncated', Buffer.from('{"tasks":[', 'utf8')]
  ])('quarantines %s task data with original bytes and returns backup path', async (_name, bytes) => {
    const paths = createPaths(tempRoot);
    await fs.writeFile(paths.tasksFile, bytes);
    const harness = createHarness(paths);

    const loaded = await harness.invoke('load-tasks');
    const backupPath = path.join(
      tempRoot,
      'tasks.json.corrupt-2026-07-24T12-34-56-789Z'
    );

    expect(loaded).toEqual({
      tasks: [],
      currentTemplate: null,
      snapshots: [],
      corruptBackupPath: backupPath
    });
    await expect(fs.readFile(backupPath)).resolves.toEqual(bytes);
    await expect(readJson(paths.tasksFile)).resolves.toEqual({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
  });

  test.each([
    ['load-templates', 'templatesFile', []],
    ['load-settings', 'settingsFile', null]
  ] as const)('quarantines corrupt data for %s and installs its live fallback', async (
    channel,
    pathKey,
    expected
  ) => {
    const paths = createPaths(tempRoot);
    const bytes = Buffer.from('{', 'utf8');
    await fs.writeFile(paths[pathKey], bytes);
    const logger = {
      log: vi.fn(),
      error: vi.fn()
    };
    const harness = createHarness(paths, logger);
    const backupPath = `${paths[pathKey]}.corrupt-2026-07-24T12-34-56-789Z`;

    await expect(harness.invoke(channel)).resolves.toEqual(expected);
    await expect(fs.readFile(backupPath)).resolves.toEqual(bytes);
    await expect(readJson(paths[pathKey])).resolves.toEqual(expected);
    expect(logger.error).toHaveBeenCalledWith(
      'Preserved corrupt data file:',
      paths[pathKey],
      '->',
      backupPath
    );
  });

  test.each([
    [
      'tasks',
      'load-tasks',
      'save-tasks',
      'tasksFile',
      { tasks: [{ text: 'replacement' }], currentTemplate: null, snapshots: [] }
    ],
    ['templates', 'load-templates', 'save-templates', 'templatesFile', [{ name: 'replacement' }]],
    ['settings', 'load-settings', 'save-settings', 'settingsFile', { transparency: 50 }]
  ] as const)('blocks %s saves after backup failure without changing source bytes', async (
    _domain,
    loadChannel,
    saveChannel,
    pathKey,
    replacement
  ) => {
    const paths = createPaths(tempRoot);
    const bytes = Buffer.from('{', 'utf8');
    await fs.writeFile(paths[pathKey], bytes);
    vi.spyOn(fs, 'copyFile').mockRejectedValueOnce(new Error('backup failed'));
    const openSpy = vi.spyOn(fs, 'open');
    const harness = createHarness(paths);

    await expect(harness.invoke(loadChannel)).rejects.toThrow('backup failed');
    await expect(harness.invoke(saveChannel, replacement)).resolves.toEqual({
      success: false,
      error: expect.stringContaining('backup failed')
    });
    expect(openSpy).not.toHaveBeenCalled();
    await expect(fs.readFile(paths[pathKey])).resolves.toEqual(bytes);
  });

  test('blocks writes when live fallback installation fails after preserving corrupt bytes', async () => {
    const paths = createPaths(tempRoot);
    const bytes = Buffer.from('{', 'utf8');
    await fs.writeFile(paths.tasksFile, bytes);
    const openSpy = vi.spyOn(fs, 'open')
      .mockRejectedValueOnce(new Error('fallback write failed'));
    const harness = createHarness(paths);
    const backupPath = `${paths.tasksFile}.corrupt-2026-07-24T12-34-56-789Z`;

    await expect(harness.invoke('load-tasks')).rejects.toThrow('fallback write failed');
    await expect(fs.readFile(backupPath)).resolves.toEqual(bytes);
    await expect(fs.readFile(paths.tasksFile)).resolves.toEqual(bytes);
    await expect(harness.invoke('save-tasks', {
      tasks: [{ text: 'replacement' }],
      currentTemplate: null,
      snapshots: []
    })).resolves.toEqual({
      success: false,
      error: expect.stringContaining('fallback write failed')
    });
    expect(openSpy).toHaveBeenCalledOnce();

    const secondHarness = createHarness(paths);
    await expect(secondHarness.invoke('load-tasks')).resolves.toEqual({
      tasks: [],
      currentTemplate: null,
      snapshots: [],
      corruptBackupPath: backupPath
    });
    await expect(fs.readFile(backupPath)).resolves.toEqual(bytes);
    await expect(readJson(paths.tasksFile)).resolves.toEqual({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
  });

  test('successful load clears an earlier write block', async () => {
    const paths = createPaths(tempRoot);
    await fs.writeFile(paths.tasksFile, '{', 'utf8');
    vi.spyOn(fs, 'copyFile').mockRejectedValueOnce(new Error('backup failed'));
    const harness = createHarness(paths);

    await expect(harness.invoke('load-tasks')).rejects.toThrow('backup failed');

    const recovered = { tasks: [], currentTemplate: null, snapshots: [] };
    await fs.writeFile(paths.tasksFile, JSON.stringify(recovered), 'utf8');
    await expect(harness.invoke('load-tasks')).resolves.toEqual(recovered);
    await expect(harness.invoke('save-tasks', recovered)).resolves.toEqual({ success: true });
    await expect(readJson(paths.tasksFile)).resolves.toEqual(recovered);
  });
});

function createPaths(root: string) {
  return {
    tasksFile: path.join(root, 'tasks.json'),
    templatesFile: path.join(root, 'templates.json'),
    settingsFile: path.join(root, 'settings.json')
  };
}

function createHarness(
  paths: ReturnType<typeof createPaths>,
  logger: typeof silentLogger = silentLogger
) {
  const handlers = new Map<string, (...args: unknown[]) => unknown>();
  const ipcMain = {
    handle: vi.fn((channel: string, handler: (...args: unknown[]) => unknown) => {
      handlers.set(channel, handler);
    })
  };

  registerStorageIpc(
    ipcMain as unknown as Parameters<typeof registerStorageIpc>[0],
    paths,
    logger
  );

  return {
    invoke: async (channel: string, ...args: unknown[]) => {
      const handler = handlers.get(channel);
      if (!handler) {
        throw new Error(`Missing IPC handler: ${channel}`);
      }
      return handler({}, ...args);
    }
  };
}

const silentLogger = {
  log: () => undefined,
  error: () => undefined
};

async function readJson(filePath: string): Promise<unknown> {
  return JSON.parse(await fs.readFile(filePath, 'utf8')) as unknown;
}
