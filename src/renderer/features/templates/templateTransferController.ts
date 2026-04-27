import type { TaskTemplate } from '../../../shared/types';
import { getErrorMessage } from '../../../shared/errors';
import type { FocusOverlayApi } from '../../ui/windowFocus';
import {
  closeImportTemplateModal,
  downloadJsonFile,
  readFileText,
  readFirstSelectedFile,
  readImportTemplateInput,
  readSelectedTemplateId,
  showImportTemplateModal,
  writeImportTemplateInput
} from './templateUi';
import {
  countTemplateTasks,
  createImportedTemplate,
  createTemplateExportData,
  createTemplateExportFilename,
  findTemplateById,
  findTemplateByName,
  parseTemplateId,
  upsertTemplateByName
} from './templateOperations';
import { parseTemplateImport } from '../tasks/taskTemplates';
import {
  denyConfirm,
  ignoreAlert,
  type AlertHandler,
  type ConfirmHandler
} from '../../ui/dialogTypes';

type LogSink = Pick<Console, 'log' | 'error'>;

export interface TemplateTransferControllerOptions {
  alertUser?: AlertHandler;
  confirmUser?: ConfirmHandler;
  api?: FocusOverlayApi;
  getTemplates: () => TaskTemplate[];
  logger?: LogSink;
  saveTemplates: (templates: TaskTemplate[]) => Promise<boolean>;
}

export interface TemplateTransferController {
  closeImportModal: () => void;
  exportTemplate: () => void;
  handleFileImport: (event: Event) => Promise<void>;
  importTemplate: () => Promise<void>;
  showImportModal: () => void;
}

export function createTemplateTransferController(
  options: TemplateTransferControllerOptions
): TemplateTransferController {
  const logger = options.logger || console;
  const alertUser = options.alertUser || ignoreAlert;
  const confirmUser = options.confirmUser || denyConfirm;

  function exportTemplate(): void {
    try {
      logger.log('Export template called');
      const templateId = parseTemplateId(readSelectedTemplateId());

      if (!templateId) {
        void alertUser('Please select a template to export!');
        return;
      }

      const template = findTemplateById(options.getTemplates(), templateId);
      if (!template) {
        return;
      }

      downloadJsonFile(createTemplateExportFilename(template.name), createTemplateExportData(template));
      void alertUser(`Template "${template.name}" exported successfully!`);
    } catch (error) {
      logger.error('Export error:', error);
      void alertUser('Error exporting template. Please try again.');
    }
  }

  function closeImportModal(): void {
    try {
      closeImportTemplateModal();
    } catch (error) {
      logger.error('Error closing import modal:', error);
    }
  }

  async function handleFileImport(event: Event): Promise<void> {
    try {
      const file = readFirstSelectedFile(event);
      if (!file) {
        return;
      }

      writeImportTemplateInput(await readFileText(file));
    } catch (error) {
      logger.error('Error handling file import:', error);
      await alertUser('Error reading file. Please try again.');
    }
  }

  async function importTemplate(): Promise<void> {
    const inputData = readImportTemplateInput();

    if (!inputData) {
      await alertUser('Please paste template JSON, share code, or select a file!');
      return;
    }

    try {
      const templateData = parseTemplateImport(inputData);
      const existingTemplate = findTemplateByName(options.getTemplates(), templateData.name);
      if (existingTemplate && !await confirmUser(`Template "${templateData.name}" already exists. Replace it?`)) {
        return;
      }

      const newTemplate = createImportedTemplate(templateData);
      const nextTemplates = upsertTemplateByName(options.getTemplates(), newTemplate);
      if (!await options.saveTemplates(nextTemplates)) {
        await alertUser(`Failed to import template "${templateData.name}". Please try again.`);
        return;
      }

      closeImportModal();
      await alertUser(`Template "${templateData.name}" imported successfully! (${countTemplateTasks(newTemplate.tasks)} tasks)`);
    } catch (error) {
      await alertUser(`Invalid template format: ${getErrorMessage(error)}`);
      logger.error('Import error:', error);
    }
  }

  return {
    closeImportModal,
    exportTemplate,
    handleFileImport,
    importTemplate,
    showImportModal: () => showImportTemplateModal(options.api)
  };
}
