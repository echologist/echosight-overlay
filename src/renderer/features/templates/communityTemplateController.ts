import type { TaskTemplate } from '../../../shared/types';
import {
  COMMUNITY_TEMPLATES,
  createCommunityTemplateTasks
} from './communityTemplates';
import {
  closeCommunityTemplatesModal,
  createCommunityTemplatePreview,
  showCommunityTemplatesModal
} from './communityTemplatesUi';
import {
  createCommunityTemplate,
  findTemplateByName,
  upsertTemplateByName
} from './templateOperations';
import {
  focusOverlaySoon,
  type FocusOverlayApi
} from '../../ui/windowFocus';
import {
  denyConfirm,
  ignoreAlert,
  type AlertHandler,
  type ConfirmHandler
} from '../../ui/dialogTypes';

type LogSink = Pick<Console, 'log' | 'error'>;

export interface CommunityTemplateControllerOptions {
  api: FocusOverlayApi;
  alertUser?: AlertHandler;
  confirmUser?: ConfirmHandler;
  getTemplates: () => TaskTemplate[];
  logger?: LogSink;
  saveTemplates: (templates: TaskTemplate[]) => Promise<boolean>;
}

export interface CommunityTemplateController {
  closeCommunityModal: () => void;
  importCommunityTemplate: (index: number) => Promise<void>;
  loadCommunityTemplates: () => void;
  previewCommunityTemplate: (index: number) => void;
}

export function createCommunityTemplateController(
  options: CommunityTemplateControllerOptions
): CommunityTemplateController {
  const logger = options.logger || console;
  const alertUser = options.alertUser || ignoreAlert;
  const confirmUser = options.confirmUser || denyConfirm;

  function loadCommunityTemplates(): void {
    try {
      logger.log('Loading community templates');
      showCommunityTemplatesModal(COMMUNITY_TEMPLATES, {
        onImport: importCommunityTemplate,
        onPreview: previewCommunityTemplate
      });
    } catch (error) {
      logger.error('Error loading community templates:', error);
      void alertUser('Error loading community templates. Please try again.');
    }
  }

  function closeCommunityModal(): void {
    try {
      closeCommunityTemplatesModal();
    } catch (error) {
      logger.error('Error closing community modal:', error);
    }
  }

  async function importCommunityTemplate(index: number): Promise<void> {
    try {
      const template = COMMUNITY_TEMPLATES[index];
      if (!template) {
        return;
      }

      const existingTemplate = findTemplateByName(options.getTemplates(), template.name);
      if (existingTemplate && !await confirmUser(`Template "${template.name}" already exists. Replace it?`)) {
        return;
      }

      const newTemplate = createCommunityTemplate(template, createCommunityTemplateTasks(template));
      const nextTemplates = upsertTemplateByName(options.getTemplates(), newTemplate);
      if (!await options.saveTemplates(nextTemplates)) {
        await alertUser(`Failed to import community template "${template.name}". Please try again.`);
        return;
      }

      await alertUser(`Community template "${template.name}" added to your templates!`);
      focusOverlaySoon(options.api);
    } catch (error) {
      logger.error('Error importing community template:', error);
      await alertUser('Error importing template. Please try again.');
    }
  }

  function previewCommunityTemplate(index: number): void {
    try {
      const template = COMMUNITY_TEMPLATES[index];
      if (!template) {
        return;
      }

      void alertUser(createCommunityTemplatePreview(template));
    } catch (error) {
      logger.error('Error previewing template:', error);
      void alertUser('Error previewing template.');
    }
  }

  return {
    closeCommunityModal,
    importCommunityTemplate,
    loadCommunityTemplates,
    previewCommunityTemplate
  };
}
