import type { HotkeyRecorder } from '../features/settings/hotkeyRecorder';
import {
  bindClick,
  bindEnterKey,
  bindEvent,
  bindValueChange,
  bindValueInput,
  clickElement,
  isOutsideModalContent
} from './domEvents';
import {
  setupGlobalErrorLogging,
  setupHeaderWindowDrag
} from './overlayUi';

type CommandHandler = () => void | Promise<void>;
type ValueHandler = (value: string) => void | Promise<void>;
type EventHandler = (event: Event) => void | Promise<void>;
type LogSink = Pick<Console, 'log' | 'error'>;

export interface RendererControlHandlers {
  addNewBackgroundTaskFromTriggerModal: CommandHandler;
  addTask: CommandHandler;
  clearAllTasks: CommandHandler;
  closeCommunityModal: CommandHandler;
  closeImportModal: CommandHandler;
  closeOverlay: CommandHandler;
  closeSaveTemplateModal: CommandHandler;
  closeSettingsModal: CommandHandler;
  closeSubTaskModal: CommandHandler;
  closeTriggersModal: CommandHandler;
  deleteTemplate: CommandHandler;
  exportTemplate: CommandHandler;
  handleFileImport: EventHandler;
  hotkeyRecorder: Pick<HotkeyRecorder, 'handleKeydown' | 'isRecording' | 'record' | 'stop'>;
  importTemplate: CommandHandler;
  loadCommunityTemplates: CommandHandler;
  loadTemplate: CommandHandler;
  logger?: LogSink;
  minimizeOverlay: CommandHandler;
  openThemesFolder: CommandHandler;
  reloadThemes: CommandHandler;
  resetHotkeys: CommandHandler;
  resetPosition: CommandHandler;
  saveSettings: CommandHandler;
  saveSubTask: CommandHandler;
  saveTemplate: CommandHandler;
  saveTriggers: CommandHandler;
  showImportModal: CommandHandler;
  showSaveTemplateModal: CommandHandler;
  showSettingsModal: CommandHandler;
  toggleInteractiveMode: CommandHandler;
  updateTheme: ValueHandler;
  updateTransparency: ValueHandler;
}

export function setupRendererControls(handlers: RendererControlHandlers): void {
  const logger = handlers.logger || console;

  bindClick('settingsButton', handlers.showSettingsModal);
  bindClick('minimizeButton', handlers.minimizeOverlay);
  bindClick('closeButton', handlers.closeOverlay);

  bindClick('loadTemplateButton', handlers.loadTemplate);
  bindClick('showSaveTemplateButton', handlers.showSaveTemplateModal);
  bindClick('exportTemplateButton', handlers.exportTemplate);
  bindClick('showImportButton', handlers.showImportModal);
  bindClick('deleteTemplateButton', handlers.deleteTemplate);
  bindClick('clearTasksButton', handlers.clearAllTasks);
  bindClick('communityTemplatesButton', handlers.loadCommunityTemplates);
  bindClick('interactiveToggle', handlers.toggleInteractiveMode);
  bindClick('addTaskButton', handlers.addTask);

  bindClick('cancelSaveTemplateButton', handlers.closeSaveTemplateModal);
  bindClick('saveTemplateButton', handlers.saveTemplate);
  bindClick('chooseImportFileButton', () => clickElement('importFileInput'));
  bindClick('cancelImportButton', handlers.closeImportModal);
  bindClick('importTemplateConfirmButton', handlers.importTemplate);
  bindClick('closeCommunityButton', handlers.closeCommunityModal);

  bindClick('openThemesFolderButton', handlers.openThemesFolder);
  bindClick('reloadThemesButton', handlers.reloadThemes);
  bindClick('recordBtn1', () => handlers.hotkeyRecorder.record('toggleVisibility'));
  bindClick('recordBtn2', () => handlers.hotkeyRecorder.record('toggleInteractive'));
  bindClick('recordBtn3', () => handlers.hotkeyRecorder.record('completeNextTask'));
  bindClick('recordBtn4', () => handlers.hotkeyRecorder.record('undoLastAction'));
  bindClick('recordBtn5', () => handlers.hotkeyRecorder.record('redoLastAction'));
  bindClick('resetHotkeysButton', handlers.resetHotkeys);
  bindClick('resetPositionButton', handlers.resetPosition);
  bindClick('closeSettingsButton', handlers.closeSettingsModal);
  bindClick('saveSettingsButton', handlers.saveSettings);

  bindClick('closeSubTaskButton', handlers.closeSubTaskModal);
  bindClick('saveSubTaskButton', handlers.saveSubTask);
  bindClick('addNewBgTaskButton', handlers.addNewBackgroundTaskFromTriggerModal);
  bindClick('closeTriggersButton', handlers.closeTriggersModal);
  bindClick('saveTriggersButton', handlers.saveTriggers);

  bindValueInput('transparencySlider', handlers.updateTransparency);
  bindValueChange('themeSelect', handlers.updateTheme);
  bindEvent('importFileInput', 'change', handlers.handleFileImport);

  bindEnterKey('taskInput', handlers.addTask);
  bindEnterKey('importTemplateInput', handlers.importTemplate, { primaryModifier: true });
  bindEnterKey('subTaskInput', handlers.saveSubTask);
  bindEnterKey('newBgTaskInput', handlers.addNewBackgroundTaskFromTriggerModal);
  bindEnterKey('templateNameInput', handlers.saveTemplate);

  window.addEventListener('keydown', handlers.hotkeyRecorder.handleKeydown, true);

  bindEvent('taskInput', 'focus', () => {
    logger.log('Task input gained focus');
  });

  bindEvent('taskInput', 'blur', () => {
    logger.log('Task input lost focus');
  });

  document.addEventListener('click', event => {
    if (handlers.hotkeyRecorder.isRecording() && isOutsideModalContent(event)) {
      handlers.hotkeyRecorder.stop();
    }
  });

  setupHeaderWindowDrag();
  setupGlobalErrorLogging(logger);
}
