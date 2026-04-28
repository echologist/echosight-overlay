import {
  setupRendererControls
} from './ui/staticControls.ts';
import {
  minimizeOverlayWindow,
  quitOverlayApplication,
  resetOverlayPosition,
  toggleOverlayInteractiveMode
} from './ui/windowCommands.ts';
import {
  createOverlayController,
  type OverlayController
} from './ui/overlayController.ts';
import { showBackgroundTaskNotification } from './features/tasks/backgroundTaskUi.ts';
import { createBackgroundTaskController } from './features/tasks/backgroundTaskController.ts';
import { createTriggerController } from './features/tasks/triggerController.ts';
import { createTemplateController } from './features/templates/templateController.ts';
import {
  createSettingsController,
  type SettingsController
} from './features/settings/settingsController.ts';
import { createThemeController } from './features/themes/themeController.ts';
import { showTaskContextMenu } from './features/tasks/contextMenuUi.ts';
import { createTaskDragController } from './features/tasks/taskDragController.ts';
import { renderTaskLists } from './features/tasks/taskListView.ts';
import {
  renderTaskProgress
} from './features/tasks/taskProgressUi.ts';
import { createSubtaskController } from './features/tasks/subtaskController.ts';
import {
  createTaskStateController
} from './features/tasks/taskStateController.ts';
import {
  createTaskWorkflowController,
  type TaskWorkflowController
} from './features/tasks/taskWorkflowController.ts';
import {
  loadTemplateState
} from './features/templates/templatePersistence.ts';
import {
  createDialogService
} from './ui/dialogService.ts';
import type {
  EchosightApi,
  TaskTemplate
} from '../shared/types';

const ipc = getRequiredEchosightApi();
const dialogs = createDialogService();

const taskController = createTaskStateController({
  api: ipc,
  logger: console
});
let templates: TaskTemplate[] = [];
const backgroundTaskController = createBackgroundTaskController({
  getTasks: () => taskController.getTasks(),
  onChanged: () => {
    renderTasks();
    updateProgress();
    saveTasks();
  },
  onActivated: showBackgroundTaskNotification,
  logger: console
});
let settingsController: SettingsController;
let overlayController: OverlayController;
let taskWorkflowController: TaskWorkflowController;
const taskDragController = createTaskDragController({
  getTasks: () => taskController.getTasks(),
  onReorder: (draggedId, targetId, insertAbove, makeSubtask) =>
    taskWorkflowController.reorderTasksAdvanced(draggedId, targetId, insertAbove, makeSubtask),
  logger: console
});
taskWorkflowController = createTaskWorkflowController({
  alertUser: dialogs.alert,
  backgroundTasks: backgroundTaskController,
  confirmUser: dialogs.confirm,
  isInteractive: () => overlayController.isInteractive(),
  logger: console,
  renderTasks,
  saveTasks,
  taskState: taskController,
  updateProgress
});
const triggerController = createTriggerController({
  api: ipc,
  captureUndoState: taskController.captureUndoState,
  getTasks: () => taskController.getTasks(),
  findTaskById: taskId => taskWorkflowController.findTaskById(taskId),
  addBackgroundTask: taskWorkflowController.addBackgroundTask,
  onChanged: () => {
    renderTasks();
    updateProgress();
    saveTasks();
  },
  alertUser: dialogs.alert
});
const subtaskController = createSubtaskController({
  api: ipc,
  addTask: (text, parentId) => taskController.addTask(text, parentId),
  getTasks: () => taskController.getTasks(),
  onChanged: () => {
    renderTasks();
    updateProgress();
    saveTasks();
  },
  logger: console,
  alertUser: dialogs.alert
});
const templateController = createTemplateController({
  api: ipc,
  captureUndoState: taskController.captureUndoState,
  clearBackgroundTimers: () => backgroundTaskController.clearAllExpirationTimers(),
  getTasks: () => taskController.getTasks(),
  getTemplates: () => templates,
  setTemplates: nextTemplates => {
    templates = nextTemplates;
  },
  replaceTasks: (nextTasks, nextCurrentTemplate) => {
    taskController.replaceTasks(nextTasks, nextCurrentTemplate);
    taskWorkflowController.migrateTaskStructure();
  },
  onTasksChanged: () => {
    renderTasks();
    updateProgress();
  },
  saveTasks,
  logger: console,
  alertUser: dialogs.alert,
  confirmUser: dialogs.confirm
});
overlayController = createOverlayController({
  api: ipc,
  getSettings: () => settingsController.getSettings(),
  onCompleteNextTask: taskWorkflowController.completeNextTask,
  onRedoLastTaskAction: taskWorkflowController.redoLastAction,
  onUndoLastTaskAction: taskWorkflowController.undoLastAction,
  logger: console
});
const themeController = createThemeController({
  api: ipc,
  logger: console,
  alertUser: dialogs.alert,
  afterReload: async () => {
    settingsController.updateThemeSelector();
    await settingsController.applySelectedTheme();
  }
});
settingsController = createSettingsController({
  api: ipc,
  getThemes: () => themeController.getThemes(),
  getIsInteractiveMode: () => overlayController.isInteractive(),
  onInteractiveRefresh: () => overlayController.refreshInteractiveVisuals(true),
  logger: console,
  alertUser: dialogs.alert,
  confirmUser: dialogs.confirm
});

function getRequiredEchosightApi(): EchosightApi {
  if (!window.echosight) {
    throw new Error('Echosight preload API is unavailable');
  }

  return window.echosight;
}

// Initialize app
async function initializeApp(): Promise<void> {
  try {
    console.log('Initializing app...');
    await loadTasks();
    taskWorkflowController.migrateTaskStructure();
    await loadTemplates();
    await themeController.loadThemes();
    await settingsController.loadSettings();
    templateController.updateTemplateSelect();
    renderTasks();
    taskDragController.setupContainerDropHandlers();
    updateProgress();
    overlayController.setupIpcListeners();
    await settingsController.applyTheme();
    overlayController.refreshInteractiveVisuals();
    taskWorkflowController.restartExpirationTimers();
    console.log('App initialized successfully');
  } catch (error) {
    console.error('Error initializing app:', error);
    void dialogs.alert('Error initializing app. Check console for details.');
  }
}

function configureTriggers(taskId: number): void {
  triggerController.configureTriggers(taskId);
}

// Enhanced renderTasks with hierarchical structure and background task support
function renderTasks(): void {
  const renderCallbacks = {
    onToggleTask: taskWorkflowController.toggleTask,
    onContextMenu: showContextMenu,
    onConfigureTriggers: configureTriggers,
    onDeleteTask: taskWorkflowController.deleteTask
  };

  renderTaskLists(taskController.getTasks(), renderCallbacks);
  taskDragController.initializeDragAndDrop();
}

function showContextMenu(event: MouseEvent, taskId: number, isParent: boolean): void {
  showTaskContextMenu(event, taskId, {
    onAddSubTask: subtaskController.addSubTask,
    onConfigureTriggers: configureTriggers
  });
}

function updateProgress(): void {
  renderTaskProgress(taskController.getProgress());
}

// Data persistence
async function loadTasks(): Promise<void> {
  await taskController.loadTasks();
}

async function saveTasks(): Promise<void> {
  await taskController.saveTasks();
}

async function loadTemplates(): Promise<void> {
  templates = await loadTemplateState(ipc, console);
}

// Event listeners and initialization
document.addEventListener('DOMContentLoaded', () => {
  setupRendererControls({
    addNewBackgroundTaskFromTriggerModal: triggerController.addNewBackgroundTaskFromTriggerModal,
    addTask: () => taskWorkflowController.addTask(),
    clearAllTasks: taskWorkflowController.clearAllTasks,
    closeCommunityModal: templateController.closeCommunityModal,
    closeImportModal: templateController.closeImportModal,
    closeOverlay: () => quitOverlayApplication(ipc, dialogs.confirm, console),
    closeSaveTemplateModal: templateController.closeSaveTemplateModal,
    closeSettingsModal: settingsController.closeSettingsModal,
    closeSubTaskModal: subtaskController.closeSubTaskModal,
    closeThemeSelectionModal: settingsController.closeThemeSelectionModal,
    closeTriggersModal: triggerController.closeTriggersModal,
    deleteTemplate: templateController.deleteTemplate,
    exportTemplate: templateController.exportTemplate,
    handleFileImport: templateController.handleFileImport,
    hotkeyRecorder: settingsController.hotkeyRecorder,
    importTemplate: templateController.importTemplate,
    loadCommunityTemplates: templateController.loadCommunityTemplates,
    loadTemplate: templateController.loadTemplate,
    logger: console,
    minimizeOverlay: () => minimizeOverlayWindow(ipc, console),
    openThemesFolder: themeController.openThemesFolder,
    reloadThemes: themeController.reloadThemes,
    resetHotkeys: settingsController.resetHotkeys,
    resetPosition: () => resetOverlayPosition(ipc, dialogs.alert, console),
    saveSettings: settingsController.saveSettings,
    saveSubTask: subtaskController.saveSubTask,
    saveTemplate: templateController.saveTemplate,
    saveTriggers: triggerController.saveTriggers,
    showImportModal: templateController.showImportModal,
    showSaveTemplateModal: templateController.showSaveTemplateModal,
    showSettingsModal: settingsController.showSettingsModal,
    showThemeSelection: settingsController.showThemeSelection,
    toggleInteractiveMode: () => toggleOverlayInteractiveMode(ipc, console),
    updateTheme: settingsController.updateTheme,
    updateTransparency: settingsController.updateTransparency
  });

  // Initialize the app
  initializeApp();
});
