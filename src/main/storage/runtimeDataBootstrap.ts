import { promises as fs } from 'fs';
import path from 'path';
import type { Settings, TaskSaveData } from '../../shared/types';
import { readJsonFile, writeJsonFile } from '../utils/jsonStorage';

export interface RuntimeDataBootstrapOptions {
  dataDir: string;
  defaultDataDir: string;
  settings: Settings;
  logger?: Pick<Console, 'log' | 'warn'>;
}

interface BootstrapFile {
  filename: string;
  fallback: unknown;
  source?: string;
}

const EMPTY_TASK_STATE: TaskSaveData = {
  tasks: [],
  currentTemplate: null,
  snapshots: []
};

export async function initializeRuntimeData(options: RuntimeDataBootstrapOptions): Promise<void> {
  const logger = options.logger || console;
  await fs.mkdir(options.dataDir, { recursive: true });

  const files: BootstrapFile[] = [
    {
      filename: 'current_tasks.json',
      source: path.join(options.defaultDataDir, 'current_tasks.json'),
      fallback: EMPTY_TASK_STATE
    },
    {
      filename: 'templates.json',
      source: path.join(options.defaultDataDir, 'templates.json'),
      fallback: []
    },
    {
      filename: 'settings.json',
      fallback: options.settings
    }
  ];

  for (const file of files) {
    await ensureRuntimeJsonFile(options.dataDir, file, logger);
  }
}

async function ensureRuntimeJsonFile(
  dataDir: string,
  file: BootstrapFile,
  logger: Pick<Console, 'log' | 'warn'>
): Promise<void> {
  const targetPath = path.join(dataDir, file.filename);
  if (await pathExists(targetPath)) {
    return;
  }

  const value = file.source
    ? await readBundledDefault(file.source, file.fallback, logger)
    : file.fallback;

  logger.log(`Creating runtime data file: ${targetPath}`);
  await writeJsonFile(targetPath, value);
}

async function readBundledDefault(
  sourcePath: string,
  fallback: unknown,
  logger: Pick<Console, 'warn'>
): Promise<unknown> {
  try {
    return await readJsonFile(sourcePath);
  } catch (error) {
    logger.warn(`Could not read bundled runtime default ${sourcePath}; using empty fallback`, error);
    return fallback;
  }
}

async function pathExists(filePath: string): Promise<boolean> {
  try {
    await fs.access(filePath);
    return true;
  } catch {
    return false;
  }
}
