import { createTaskManager } from './taskManager.js';
import { createTemplateManager } from './templateManager.js';
import { createThemeManager } from './themeManager.js';
import { createDragDrop } from './dragDrop.js';
import { createUiRenderer } from './uiRenderer.js';
import { createProgressTracker } from './progressTracker.js';

// ES Module imports 4 Vite
const { ipcRenderer } = window.require('electron');

const state = {
  tasks: [],
  snapshots: [],
  templates: [],
  themes: [],
  currentTemplate: null,
  isInteractiveMode: true,
  settings: {
    transparency: 60,
    theme: 'echosight',
    hotkeys: {
      toggleVisibility: 'Ctrl+Shift+T',
      toggleInteractive: 'Ctrl+Shift+I',
      completeNextTask: 'Ctrl+Shift+N',
      undoLastAction: 'Ctrl+Shift+Z'
    }
  }
};

const managers = {};
const ctx = { ipcRenderer, state, managers };

managers.progress = createProgressTracker(ctx);
managers.theme = createThemeManager(ctx);
managers.task = createTaskManager(ctx);
managers.template = createTemplateManager(ctx);
managers.dragDrop = createDragDrop(ctx);
managers.ui = createUiRenderer(ctx);

async function initializeApp() {
  try {
    console.log('Initializing app...');
    await managers.task.loadTasks();
    managers.task.migrateTaskStructure();
    await managers.template.loadTemplates();
    await managers.theme.loadThemes();
    await managers.theme.loadSettings();
    managers.template.updateTemplateSelect();
    managers.ui.renderTasks();
    managers.dragDrop.setupContainerDropHandler();
    managers.progress.updateProgress();
    managers.task.setupInteractiveModeListener();
    await managers.theme.applyTheme();
    managers.task.restartExpirationTimers();
    console.log('App initialized successfully');
  } catch (error) {
    console.error('Error initializing app:', error);
    alert('Error initializing app. Check console for details.');
  }
}

// Event listeners and initialization
document.addEventListener('DOMContentLoaded', () => {
  // Task input event listeners
  document.getElementById('taskInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter') {
      managers.task.addTask();
    }
  });

  document.getElementById('importTemplateInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter' && e.ctrlKey) {
      managers.template.importTemplate();
    }
  });

  // Subtask input event listeners
  document.getElementById('subTaskInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter') {
      managers.ui.saveSubTask();
    }
  });

  // Background task input in triggers modal
  const bgTaskInput = document.getElementById('newBgTaskInput');
  if (bgTaskInput) {
    bgTaskInput.addEventListener('keypress', function (e) {
      if (e.key === 'Enter') {
        managers.task.addNewBackgroundTaskFromTriggerModal();
      }
    });
  }

  // Global keydown listener for hotkey recording
  document.addEventListener('keydown', managers.theme.handleHotkeyRecording);

  document.getElementById('taskInput').addEventListener('focus', function () {
    console.log('Task input gained focus');
  });

  document.getElementById('taskInput').addEventListener('blur', function () {
    console.log('Task input lost focus');
  });

  document.getElementById('templateNameInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter') {
      managers.template.saveTemplate();
    }
  });

  document.getElementById('importTemplateInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter' && e.ctrlKey) {
      managers.template.importTemplate();
    }
  });

  // Global keydown listener for hotkey recording
  document.addEventListener('keydown', managers.theme.handleHotkeyRecording);

  // Click outside modal to stop recording
  document.addEventListener('click', function (e) {
    if (window.recordingHotkey && !e.target.closest('.modal-content')) {
      managers.theme.stopRecording();
    }
  });

  // Make header draggable
  const header = document.getElementById('header');
  let isDragging = false;
  let dragOffset = { x: 0, y: 0 };

  header.addEventListener('mousedown', (e) => {
    try {
      console.log('Header mousedown');
      isDragging = true;
      const rect = header.getBoundingClientRect();
      dragOffset.x = e.clientX - rect.left;
      dragOffset.y = e.clientY - rect.top;
    } catch (error) {
      console.error('Error in mousedown:', error);
    }
  });

  document.addEventListener('mousemove', (e) => {
    try {
      if (isDragging) {
        const newX = e.screenX - dragOffset.x;
        const newY = e.screenY - dragOffset.y;

        // Send move request to main process
        ipcRenderer.send('move-window', { x: newX, y: newY });
      }
    } catch (error) {
      console.error('Error in mousemove:', error);
      isDragging = false;
    }
  });

  document.addEventListener('mouseup', () => {
    try {
      isDragging = false;
    } catch (error) {
      console.error('Error in mouseup:', error);
    }
  });

  // Debug: Log when overlay gets focus/blur - temporarily disabled during drag debugging
  // window.addEventListener('focus', () => {
  //   console.log('Overlay gained focus');
  // });

  // window.addEventListener('blur', () => {
  //   console.log('Overlay lost focus');
  // });

  // Debug function - call from console to test input
  window.testInput = function () {
    const input = document.getElementById('taskInput');
    input.focus();
    input.value = 'Test task';
    console.log('Test input set. Try typing or clicking Add.');
  };

  // Debug function - call from console to test buttons
  window.testMinimize = function () {
    console.log('Testing minimize...');
    managers.ui.minimizeOverlay();
  };

  // Global error handler
  window.addEventListener('error', function (e) {
    console.error('Global error:', e.error);
    console.error('Error message:', e.message);
    console.error('Error location:', e.filename, e.lineno, e.colno);
  });

  window.addEventListener('unhandledrejection', function (e) {
    console.error('Unhandled promise rejection:', e.reason);
  });

  // Initialize the app
  initializeApp();
});

// make functions global for onclick handlers
Object.assign(window, {
  addTask: managers.task.addTask,
  toggleTask: managers.task.toggleTask,
  undoLastAction: managers.task.undoLastAction,
  deleteTask: managers.task.deleteTask,
  showContextMenu: managers.ui.showContextMenu,
  addSubTask: managers.ui.addSubTask,
  clearAllTasks: managers.task.clearAllTasks,
  closeSubTaskModal: managers.ui.closeSubTaskModal,
  saveSubTask: managers.ui.saveSubTask,
  minimizeOverlay: managers.ui.minimizeOverlay,
  closeOverlay: managers.ui.closeOverlay,
  toggleInteractiveMode: managers.ui.toggleInteractiveMode,
  showSettingsModal: managers.theme.showSettingsModal,
  closeSettingsModal: managers.theme.closeSettingsModal,
  resetHotkeys: managers.theme.resetHotkeys,
  updateTransparency: managers.theme.updateTransparency,
  updateTheme: managers.theme.updateTheme,
  recordHotkey: managers.theme.recordHotkey,
  resetPosition: managers.theme.resetPosition,
  saveSettings: managers.theme.saveSettings,
  showSaveTemplateModal: managers.template.showSaveTemplateModal,
  closeSaveTemplateModal: managers.template.closeSaveTemplateModal,
  saveTemplate: managers.template.saveTemplate,
  loadTemplate: managers.template.loadTemplate,
  deleteTemplate: managers.template.deleteTemplate,
  exportTemplate: managers.template.exportTemplate,
  showImportModal: managers.template.showImportModal,
  closeImportModal: managers.template.closeImportModal,
  importTemplate: managers.template.importTemplate,
  handleFileImport: managers.template.handleFileImport,
  loadCommunityTemplates: managers.template.loadCommunityTemplates,
  closeCommunityModal: managers.template.closeCommunityModal,
  importCommunityTemplate: managers.template.importCommunityTemplate,
  previewCommunityTemplate: managers.template.previewCommunityTemplate,
  openThemesFolder: managers.theme.openThemesFolder,
  reloadThemes: managers.theme.reloadThemes,
  showThemesPath: managers.theme.showThemesPath,
  configureTriggers: managers.task.configureTriggers,
  closeTriggersModal: managers.task.closeTriggersModal,
  saveTriggers: managers.task.saveTriggers,
  addNewBackgroundTaskFromTriggerModal: managers.task.addNewBackgroundTaskFromTriggerModal,
  dismissBackgroundTask: managers.task.dismissBackgroundTask
});
