import { promises as fs } from 'fs';
import os from 'os';
import path from 'path';
import { afterEach, beforeEach, describe, expect, test } from 'vitest';
import { initializeRuntimeData } from '../../../src/main/storage/runtimeDataBootstrap';
import type { Settings } from '../../../src/shared/types';

describe('runtime data bootstrap', () => {
  let tempRoot = '';

  beforeEach(async () => {
    tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'echosight-runtime-'));
  });

  afterEach(async () => {
    if (tempRoot) {
      await fs.rm(tempRoot, { recursive: true, force: true });
    }
  });

  test('creates missing runtime files in userData from bundled defaults', async () => {
    const paths = await createRuntimePaths();
    const taskSeed = {
      tasks: [{ text: 'Welcome' }],
      currentTemplate: 'Starter'
    };
    const templateSeed = [{
      id: 1,
      name: 'Starter',
      tasks: [{ text: 'Welcome' }],
      createdAt: '2026-04-28T00:00:00.000Z'
    }];

    await writeJson(paths.defaultDataDir, 'current_tasks.json', taskSeed);
    await writeJson(paths.defaultDataDir, 'templates.json', templateSeed);

    await initializeRuntimeData({
      dataDir: paths.dataDir,
      defaultDataDir: paths.defaultDataDir,
      settings: settingsFixture,
      logger: silentLogger
    });

    await expect(readJson(paths.dataDir, 'current_tasks.json')).resolves.toEqual(taskSeed);
    await expect(readJson(paths.dataDir, 'templates.json')).resolves.toEqual(templateSeed);
    await expect(readJson(paths.dataDir, 'settings.json')).resolves.toEqual(settingsFixture);
  });

  test('does not overwrite existing runtime files', async () => {
    const paths = await createRuntimePaths();
    const existingTasks = {
      tasks: [{ text: 'Existing' }],
      currentTemplate: null,
      snapshots: []
    };

    await writeJson(paths.dataDir, 'current_tasks.json', existingTasks);
    await writeJson(paths.defaultDataDir, 'current_tasks.json', {
      tasks: [{ text: 'Bundled' }],
      currentTemplate: null
    });

    await initializeRuntimeData({
      dataDir: paths.dataDir,
      defaultDataDir: paths.defaultDataDir,
      settings: settingsFixture,
      logger: silentLogger
    });

    await expect(readJson(paths.dataDir, 'current_tasks.json')).resolves.toEqual(existingTasks);
  });

  test('uses empty fallbacks when bundled defaults are missing', async () => {
    const paths = await createRuntimePaths();

    await initializeRuntimeData({
      dataDir: paths.dataDir,
      defaultDataDir: paths.defaultDataDir,
      settings: settingsFixture,
      logger: silentLogger
    });

    await expect(readJson(paths.dataDir, 'current_tasks.json')).resolves.toEqual({
      tasks: [],
      currentTemplate: null,
      snapshots: []
    });
    await expect(readJson(paths.dataDir, 'templates.json')).resolves.toEqual([]);
  });

  async function createRuntimePaths() {
    const dataDir = path.join(tempRoot, 'user-data', 'data');
    const defaultDataDir = path.join(tempRoot, 'defaults');
    await fs.mkdir(dataDir, { recursive: true });
    await fs.mkdir(defaultDataDir, { recursive: true });
    return { dataDir, defaultDataDir };
  }
});

const settingsFixture: Settings = {
  settingsVersion: 1,
  transparency: 70,
  theme: 'echosight',
  hotkeys: {
    toggleVisibility: 'CommandOrControl+Shift+T',
    toggleInteractive: 'CommandOrControl+Shift+I',
    completeNextTask: 'CommandOrControl+Shift+N',
    undoLastAction: 'CommandOrControl+Shift+Z',
    redoLastAction: 'CommandOrControl+Shift+Y'
  }
};

async function writeJson(directory: string, filename: string, value: unknown): Promise<void> {
  await fs.mkdir(directory, { recursive: true });
  await fs.writeFile(path.join(directory, filename), JSON.stringify(value, null, 2), 'utf8');
}

async function readJson(directory: string, filename: string): Promise<unknown> {
  return JSON.parse(await fs.readFile(path.join(directory, filename), 'utf8')) as unknown;
}

const silentLogger = {
  log: () => undefined,
  warn: () => undefined
};
