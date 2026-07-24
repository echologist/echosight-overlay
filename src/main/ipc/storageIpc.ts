import { promises as fs } from 'fs';
import type { IpcMain } from 'electron';
import { getErrorMessage } from '../../shared/errors';
import { parseJson, writeJsonFile } from '../utils/jsonStorage';

export interface StorageIpcPaths {
  tasksFile: string;
  templatesFile: string;
  settingsFile: string;
}

type LogSink = Pick<Console, 'log' | 'error'>;
const EMPTY_TASK_STATE = { tasks: [], currentTemplate: null, snapshots: [] };

export function registerStorageIpc(
  ipcMain: IpcMain,
  paths: StorageIpcPaths,
  logger: LogSink = console
): void {
  const blockedPaths = new Map<string, string>();
  const load = async <T>(filePath: string, fallback: T) => {
    try {
      const loaded = await loadJsonOrQuarantine(filePath, fallback);
      if (loaded.corruptBackupPath) {
        logger.error(
          'Preserved corrupt data file:',
          filePath,
          '->',
          loaded.corruptBackupPath
        );
      }
      blockedPaths.delete(filePath);
      return loaded;
    } catch (error) {
      blockedPaths.set(filePath, getErrorMessage(error));
      throw error;
    }
  };
  const getSaveBlock = (filePath: string) => {
    const error = blockedPaths.get(filePath);
    return error
      ? { success: false, error: `Save blocked after load failure: ${error}` }
      : null;
  };

  ipcMain.handle('load-tasks', async () => {
    const loaded = await load(paths.tasksFile, EMPTY_TASK_STATE);
    if (loaded.corruptBackupPath) {
      return {
        ...EMPTY_TASK_STATE,
        corruptBackupPath: loaded.corruptBackupPath
      };
    }

    return loaded.value;
  });

  ipcMain.handle('save-tasks', async (_event, tasksData) => {
    const blocked = getSaveBlock(paths.tasksFile);
    if (blocked) {
      return blocked;
    }

    try {
      await writeJsonFile(paths.tasksFile, tasksData);
      return { success: true };
    } catch (error) {
      logger.error('Failed to save tasks:', error);
      return { success: false, error: getErrorMessage(error) };
    }
  });

  ipcMain.handle('load-templates', async () => {
    return (await load(paths.templatesFile, [])).value;
  });

  ipcMain.handle('save-templates', async (_event, templates) => {
    const blocked = getSaveBlock(paths.templatesFile);
    if (blocked) {
      return blocked;
    }

    try {
      await writeJsonFile(paths.templatesFile, templates);
      return { success: true };
    } catch (error) {
      logger.error('Failed to save templates:', error);
      return { success: false, error: getErrorMessage(error) };
    }
  });

  ipcMain.handle('save-settings', async (_event, settings) => {
    const blocked = getSaveBlock(paths.settingsFile);
    if (blocked) {
      return blocked;
    }

    try {
      logger.log('Attempting to save settings:', settings);
      logger.log('Settings file path:', paths.settingsFile);

      await writeJsonFile(paths.settingsFile, settings);
      logger.log('Settings saved successfully to:', paths.settingsFile);

      return { success: true };
    } catch (error) {
      logger.error('Failed to save settings:', error);
      return { success: false, error: getErrorMessage(error) };
    }
  });

  ipcMain.handle('load-settings', async () => {
    try {
      logger.log('Attempting to load settings from:', paths.settingsFile);
      const settings = (await load(paths.settingsFile, null)).value;
      logger.log('Settings loaded successfully:', settings);
      return settings;
    } catch (error) {
      logger.error('Failed to preserve unreadable settings:', getErrorMessage(error));
      throw error;
    }
  });
}

async function loadJsonOrQuarantine<T>(
  filePath: string,
  fallback: T
): Promise<{ value: T | unknown; corruptBackupPath?: string }> {
  let data: Buffer;

  try {
    data = await fs.readFile(filePath);
  } catch (error) {
    if (isFileNotFound(error)) {
      return { value: fallback };
    }
    throw error;
  }

  try {
    return { value: parseJson(data.toString('utf8')) };
  } catch {
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const corruptBackupPath = `${filePath}.corrupt-${timestamp}`;
    await fs.copyFile(filePath, corruptBackupPath);
    await writeJsonFile(filePath, fallback);
    return { value: fallback, corruptBackupPath };
  }
}

function isFileNotFound(error: unknown): boolean {
  return typeof error === 'object'
    && error !== null
    && 'code' in error
    && error.code === 'ENOENT';
}
