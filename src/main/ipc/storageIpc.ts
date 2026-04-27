import type { IpcMain } from 'electron';
import { getErrorMessage } from '../../shared/errors';
import { readJsonFile, writeJsonFile } from '../utils/jsonStorage';

export interface StorageIpcPaths {
  tasksFile: string;
  templatesFile: string;
  settingsFile: string;
}

type LogSink = Pick<Console, 'log' | 'error'>;

export function registerStorageIpc(
  ipcMain: IpcMain,
  paths: StorageIpcPaths,
  logger: LogSink = console
): void {
  ipcMain.handle('load-tasks', async () => {
    try {
      return await readJsonFile(paths.tasksFile);
    } catch {
      return { tasks: [], currentTemplate: null, snapshots: [] };
    }
  });

  ipcMain.handle('save-tasks', async (_event, tasksData) => {
    try {
      await writeJsonFile(paths.tasksFile, tasksData);
      return { success: true };
    } catch (error) {
      logger.error('Failed to save tasks:', error);
      return { success: false, error: getErrorMessage(error) };
    }
  });

  ipcMain.handle('load-templates', async () => {
    try {
      return await readJsonFile(paths.templatesFile);
    } catch {
      return [];
    }
  });

  ipcMain.handle('save-templates', async (_event, templates) => {
    try {
      await writeJsonFile(paths.templatesFile, templates);
      return { success: true };
    } catch (error) {
      logger.error('Failed to save templates:', error);
      return { success: false, error: getErrorMessage(error) };
    }
  });

  ipcMain.handle('save-settings', async (_event, settings) => {
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
      const settings = await readJsonFile(paths.settingsFile);
      logger.log('Settings loaded successfully:', settings);
      return settings;
    } catch (error) {
      logger.log('Settings file not found or invalid, using defaults:', getErrorMessage(error));
      return null;
    }
  });
}
