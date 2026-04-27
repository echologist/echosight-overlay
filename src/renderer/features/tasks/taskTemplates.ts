import { destr } from 'destr';
import type {
  Task,
  TaskMode,
  TaskTemplate,
  TemplateData,
  TemplateTask
} from '../../../shared/types';
import {
  createTaskId,
  sanitizeBackgroundOptions
} from './taskCreation';
import { isRecord } from './taskGuards';

export function normalizeTemplateText(str: unknown, maxLength = 200): string {
  if (typeof str !== 'string') return '';

  return str.trim().substring(0, maxLength);
}

export function createTemplateTasks(taskList: Task[]): TemplateTask[] {
  const taskIndexMap: Record<number, number> = {};
  let flatIndex = 0;

  function buildIndexMap(taskArray: Task[]): void {
    taskArray.filter(task => !task.completed).forEach(task => {
      taskIndexMap[task.id] = flatIndex++;
      buildIndexMap(task.children);
    });
  }

  function stripTaskForTemplate(task: Task): TemplateTask {
    const stripped: TemplateTask = { text: task.text };
    const children = task.children.filter(child => !child.completed).map(child => stripTaskForTemplate(child));

    if (children.length > 0) {
      stripped.children = children;
    }
    if (task.mode === 'background') {
      stripped.mode = 'background';
      stripped.backgroundOptions = task.backgroundOptions;
    }
    if (task.triggers.length > 0) {
      const triggerIndices = task.triggers
        .map(id => taskIndexMap[id])
        .filter((idx): idx is number => idx !== undefined);

      if (triggerIndices.length > 0) {
        stripped.triggerIndices = triggerIndices;
      }
    }
    return stripped;
  }

  buildIndexMap(taskList.filter(task => !task.completed));
  return taskList.filter(task => !task.completed).map(task => stripTaskForTemplate(task));
}

export function normalizeTemplateTask(task: unknown): TemplateTask | null {
  const text = normalizeTemplateText(
    typeof task === 'string' ? task : (isRecord(task) ? task.text : '')
  );

  if (!text) {
    return null;
  }

  const normalized: TemplateTask = { text };
  const sourceChildren = isRecord(task) && Array.isArray(task.children) ? task.children : [];
  const children = sourceChildren.map(normalizeTemplateTask).filter((child): child is TemplateTask => !!child);

  if (children.length > 0) {
    normalized.children = children;
  }

  if (isRecord(task) && task.mode === 'background') {
    normalized.mode = 'background';
  }

  if (isRecord(task) && task.backgroundOptions) {
    normalized.backgroundOptions = sanitizeBackgroundOptions(task.backgroundOptions);
  }

  if (isRecord(task) && Array.isArray(task.triggerIndices)) {
    const triggerIndices = [...new Set(
      task.triggerIndices
        .map(Number)
        .filter(index => Number.isInteger(index) && index >= 0)
    )];

    if (triggerIndices.length > 0) {
      normalized.triggerIndices = triggerIndices;
    }
  }

  return normalized;
}

export function createRuntimeTaskFromTemplate(
  templateTask: TemplateTask | string | null | undefined,
  inheritedMode: TaskMode = 'main'
): Task {
  const sourceTask = typeof templateTask === 'string'
    ? { text: templateTask }
    : templateTask || { text: '' };
  const mode = sourceTask.mode === 'background' || inheritedMode === 'background'
    ? 'background'
    : 'main';

  return {
    id: createTaskId(),
    text: String(sourceTask.text || ''),
    completed: false,
    createdAt: new Date().toISOString(),
    children: Array.isArray(sourceTask.children)
      ? sourceTask.children.map(child => createRuntimeTaskFromTemplate(child, mode))
      : [],
    mode,
    triggers: [],
    activated: mode === 'background' ? false : true,
    activatedAt: null,
    backgroundOptions: mode === 'background'
      ? sanitizeBackgroundOptions(sourceTask.backgroundOptions) || { expiresAfterMinutes: null, priority: 'normal' }
      : null
  };
}

export function hasTemplateTriggerIndices(taskArray: TemplateTask[]): boolean {
  return taskArray.some(task =>
    (Array.isArray(task.triggerIndices) && task.triggerIndices.length > 0) ||
    (Array.isArray(task.children) && hasTemplateTriggerIndices(task.children))
  );
}

export function hasBackgroundTask(taskArray: TemplateTask[]): boolean {
  return taskArray.some(task =>
    task.mode === 'background' ||
    (Array.isArray(task.children) && hasBackgroundTask(task.children))
  );
}

export function normalizeTemplateData(data: unknown): TemplateData {
  const source = isRecord(data) ? data : {};
  const sanitized: TemplateData = {
    name: normalizeTemplateText(source.name, 50) || 'Imported Template',
    tasks: []
  };

  if (Array.isArray(source.tasks)) {
    sanitized.tasks = source.tasks
      .map(normalizeTemplateTask)
      .filter((task): task is TemplateTask => !!task);
  }

  return sanitized;
}

export function normalizeSavedTemplate(template: unknown): TaskTemplate | null {
  if (!isRecord(template)) {
    return null;
  }

  const templateData = normalizeTemplateData(template);
  if (templateData.tasks.length === 0) {
    return null;
  }

  const id = Number(template.id);
  const createdAt = typeof template.createdAt === 'string' && template.createdAt.trim()
    ? template.createdAt
    : new Date().toISOString();

  return {
    id: Number.isFinite(id) ? id : createTaskId(),
    name: templateData.name,
    tasks: templateData.tasks,
    createdAt,
    ...(template.imported === true ? { imported: true } : {}),
    ...(template.community === true ? { community: true } : {})
  };
}

export function normalizeTemplateCollection(templates: unknown): TaskTemplate[] {
  if (!Array.isArray(templates)) {
    return [];
  }

  return templates
    .map(normalizeSavedTemplate)
    .filter((template): template is TaskTemplate => !!template);
}

export function parseTemplateImport(input: string, maxLength = 100000): TemplateData {
  const trimmedInput = input.trim();

  if (!trimmedInput) {
    throw new Error('Template data is empty');
  }

  if (trimmedInput.length > maxLength) {
    throw new Error('Template data too large');
  }

  const parsed = destr<unknown>(trimmedInput, { strict: true });
  if (!isRecord(parsed) || typeof parsed.name !== 'string' || !Array.isArray(parsed.tasks)) {
    throw new Error('Invalid JSON template format');
  }

  const templateData = normalizeTemplateData(parsed);
  if (templateData.tasks.length === 0) {
    throw new Error('Template must contain at least one valid task');
  }

  return templateData;
}
