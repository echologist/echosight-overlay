import type { EchosightApi, TaskTemplate } from '../../../shared/types';
import { normalizeTemplateCollection } from '../tasks/taskTemplates';

export async function loadTemplateState(
  api: EchosightApi,
  logger: Pick<Console, 'error'> = console
): Promise<TaskTemplate[]> {
  try {
    return normalizeTemplateCollection(await api.loadTemplates());
  } catch (error) {
    logger.error('Failed to load templates:', error);
    return [];
  }
}

export async function saveTemplateState(
  api: EchosightApi,
  templates: TaskTemplate[],
  logger: Pick<Console, 'error'> = console
): Promise<boolean> {
  try {
    const result = await api.saveTemplates(templates);
    if (!result.success) {
      throw new Error(result.error || 'Unknown save error');
    }

    return true;
  } catch (error) {
    logger.error('Failed to save templates:', error);
    return false;
  }
}
