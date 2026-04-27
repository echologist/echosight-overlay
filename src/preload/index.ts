import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron';
import type { EchosightApi } from '../shared/types';

const api: EchosightApi = {
  loadTasks: () => ipcRenderer.invoke('load-tasks'),
  saveTasks: (tasksData) => ipcRenderer.invoke('save-tasks', tasksData),
  loadTemplates: () => ipcRenderer.invoke('load-templates'),
  saveTemplates: (templates) => ipcRenderer.invoke('save-templates', templates),
  loadSettings: () => ipcRenderer.invoke('load-settings'),
  saveSettings: (settings) => ipcRenderer.invoke('save-settings', settings),

  loadThemes: () => ipcRenderer.invoke('load-themes'),
  getTheme: (themeId) => ipcRenderer.invoke('get-theme', themeId),
  reloadThemes: () => ipcRenderer.invoke('reload-themes'),
  openThemesFolder: () => ipcRenderer.invoke('open-themes-folder'),
  getThemesPath: () => ipcRenderer.invoke('get-themes-path'),
  loadThemeCss: (themeId, cssFileName) => ipcRenderer.invoke('load-theme-css', themeId, cssFileName),
  getThemeAsset: (themeId, assetName) => ipcRenderer.invoke('get-theme-asset', themeId, assetName),

  updateHotkeys: (hotkeys) => ipcRenderer.send('update-hotkeys', hotkeys),
  setHotkeyRecording: (recording) => ipcRenderer.send('set-hotkey-recording', recording),
  focusWindow: () => ipcRenderer.send('focus-window'),
  resetWindowPosition: () => ipcRenderer.send('reset-window-position'),
  toggleOverlay: () => ipcRenderer.send('toggle-overlay'),
  minimizeOverlay: () => ipcRenderer.send('minimize-overlay'),
  toggleInteractiveMode: () => ipcRenderer.send('toggle-interactive-mode'),
  quitApplication: () => ipcRenderer.send('quit-application'),

  onInteractiveModeChanged: (callback) => {
    const listener = (_event: IpcRendererEvent, interactive: boolean) => callback(interactive);
    ipcRenderer.on('interactive-mode-changed', listener);
    return () => ipcRenderer.removeListener('interactive-mode-changed', listener);
  },

  onCompleteNextTask: (callback) => {
    const listener = () => callback();
    ipcRenderer.on('complete-next-task', listener);
    return () => ipcRenderer.removeListener('complete-next-task', listener);
  },

  onUndoLastTaskAction: (callback) => {
    const listener = () => callback();
    ipcRenderer.on('undo-last-task-action', listener);
    return () => ipcRenderer.removeListener('undo-last-task-action', listener);
  },

  onRedoLastTaskAction: (callback) => {
    const listener = () => callback();
    ipcRenderer.on('redo-last-task-action', listener);
    return () => ipcRenderer.removeListener('redo-last-task-action', listener);
  }
};

contextBridge.exposeInMainWorld('echosight', api);
