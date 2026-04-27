import type {
  EchosightApi,
  Task,
  TaskTemplate
} from '../../../shared/types';
import { focusOverlaySoon } from '../../ui/windowFocus';
import {
  createCommunityTemplateController
} from './communityTemplateController';
import {
  closeSaveTemplateModal,
  readSelectedTemplateId,
  readTemplateName,
  renderTemplateSelect,
  showSaveTemplateModal
} from './templateUi';
import {
  createTemplateFromTasks,
  findTemplateById,
  instantiateTemplateTasks,
  parseTemplateId,
  removeTemplateById,
  upsertTemplateByName
} from './templateOperations';
import { saveTemplateState } from './templatePersistence';
import { createTemplateTransferController } from './templateTransferController';
import { hasIncompleteTasks } from '../tasks/taskCompletion';
import {
  denyConfirm,
  ignoreAlert,
  type AlertHandler,
  type ConfirmHandler
} from '../../ui/dialogTypes';

type LogSink = Pick<Console, 'log' | 'error'>;

export interface TemplateControllerOptions {
  api: EchosightApi;
  captureUndoState?: (label: string) => void;
  clearBackgroundTimers: () => void;
  getTasks: () => Task[];
  getTemplates: () => TaskTemplate[];
  logger?: LogSink;
  onTasksChanged: () => void;
  replaceTasks: (tasks: Task[], currentTemplate: string | null) => void;
  saveTasks: () => Promise<void>;
  setTemplates: (templates: TaskTemplate[]) => void;
  alertUser?: AlertHandler;
  confirmUser?: ConfirmHandler;
}

export interface TemplateController {
  closeCommunityModal: () => void;
  closeImportModal: () => void;
  closeSaveTemplateModal: () => void;
  deleteTemplate: () => Promise<void>;
  exportTemplate: () => void;
  handleFileImport: (event: Event) => Promise<void>;
  importTemplate: () => Promise<void>;
  loadCommunityTemplates: () => void;
  loadTemplate: () => Promise<void>;
  saveTemplate: () => Promise<void>;
  showImportModal: () => void;
  showSaveTemplateModal: () => void;
  updateTemplateSelect: () => void;
}

export function createTemplateController(options: TemplateControllerOptions): TemplateController {
  const logger = options.logger || console;
  const alertUser = options.alertUser || ignoreAlert;
  const confirmUser = options.confirmUser || denyConfirm;

  async function saveTemplateCollection(nextTemplates: TaskTemplate[]): Promise<boolean> {
    if (!await saveTemplateState(options.api, nextTemplates, logger)) {
      return false;
    }

    options.setTemplates(nextTemplates);
    updateTemplateSelect();
    return true;
  }

  const communityController = createCommunityTemplateController({
    api: options.api,
    alertUser,
    confirmUser,
    getTemplates: options.getTemplates,
    logger,
    saveTemplates: saveTemplateCollection
  });
  const transferController = createTemplateTransferController({
    api: options.api,
    alertUser,
    confirmUser,
    getTemplates: options.getTemplates,
    logger,
    saveTemplates: saveTemplateCollection
  });

  function updateTemplateSelect(): void {
    renderTemplateSelect(options.getTemplates());
  }

  function showSaveTemplateModalAction(): void {
    if (options.getTasks().length === 0) {
      void alertUser('No tasks to save as template!');
      return;
    }

    showSaveTemplateModal(options.api);
  }

  async function saveTemplateAction(): Promise<void> {
    const name = readTemplateName();

    if (!name) {
      void alertUser('Please enter a template name!');
      return;
    }

    const tasks = options.getTasks();
    if (tasks.length === 0) {
      void alertUser('No tasks to save!');
      return;
    }

    const template = createTemplateFromTasks(name, tasks);
    const nextTemplates = upsertTemplateByName(options.getTemplates(), template);
    if (!await saveTemplateCollection(nextTemplates)) {
      await alertUser('Failed to save template. Please try again.');
      return;
    }

    closeSaveTemplateModal();
    await alertUser(`Template "${name}" saved successfully!`);
  }

  async function loadTemplateAction(): Promise<void> {
    const templateId = parseTemplateId(readSelectedTemplateId());
    if (!templateId) {
      return;
    }

    const template = findTemplateById(options.getTemplates(), templateId);
    if (!template) {
      return;
    }

    if (hasIncompleteTasks(options.getTasks()) && !await confirmUser('This will replace your current tasks. Continue?', {
      title: 'Load Template',
      tone: 'danger'
    })) {
      return;
    }

    options.clearBackgroundTimers();
    options.captureUndoState?.('load template');
    options.replaceTasks(instantiateTemplateTasks(template), template.name);
    options.onTasksChanged();
    await options.saveTasks();
    focusOverlaySoon(options.api);
  }

  async function deleteTemplateAction(): Promise<void> {
    const templateId = parseTemplateId(readSelectedTemplateId());

    if (!templateId) {
      void alertUser('Please select a template to delete!');
      return;
    }

    const template = findTemplateById(options.getTemplates(), templateId);
    if (!template) {
      return;
    }

    if (!await confirmUser(`Are you sure you want to delete the template "${template.name}"? This cannot be undone.`, {
      title: 'Delete Template',
      tone: 'danger'
    })) {
      return;
    }

    const nextTemplates = removeTemplateById(options.getTemplates(), templateId);
    if (!await saveTemplateCollection(nextTemplates)) {
      await alertUser(`Failed to delete template "${template.name}". Please try again.`);
      return;
    }

    await alertUser(`Template "${template.name}" deleted successfully!`);
  }

  return {
    closeCommunityModal: communityController.closeCommunityModal,
    closeImportModal: transferController.closeImportModal,
    closeSaveTemplateModal,
    deleteTemplate: deleteTemplateAction,
    exportTemplate: transferController.exportTemplate,
    handleFileImport: transferController.handleFileImport,
    importTemplate: transferController.importTemplate,
    loadCommunityTemplates: communityController.loadCommunityTemplates,
    loadTemplate: loadTemplateAction,
    saveTemplate: saveTemplateAction,
    showImportModal: transferController.showImportModal,
    showSaveTemplateModal: showSaveTemplateModalAction,
    updateTemplateSelect
  };
}
