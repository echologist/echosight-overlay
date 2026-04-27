import type {
  Task,
  TaskTemplate,
  TemplateData,
  TemplateExportData,
  TemplateTask
} from '../../../shared/types';
import type { CommunityTemplate } from './communityTemplates';
import {
  createRuntimeTaskFromTemplate,
  createTemplateTasks,
  hasBackgroundTask
} from '../tasks/taskTemplates';
import { flattenTasks } from '../tasks/taskTree';

export function parseTemplateId(value: string): number | null {
  if (!value.trim()) {
    return null;
  }

  const id = Number(value);
  return Number.isFinite(id) ? id : null;
}

export function findTemplateById(templates: TaskTemplate[], templateId: number): TaskTemplate | null {
  return templates.find(template => template.id === templateId) || null;
}

export function findTemplateByName(templates: TaskTemplate[], templateName: string): TaskTemplate | null {
  return templates.find(template => template.name === templateName) || null;
}

export function createTemplateFromTasks(name: string, tasks: Task[]): TaskTemplate {
  return {
    id: Date.now(),
    name,
    tasks: createTemplateTasks(tasks),
    createdAt: new Date().toISOString()
  };
}

export function createCommunityTemplate(template: CommunityTemplate, tasks: TemplateTask[]): TaskTemplate {
  return {
    id: Date.now(),
    name: template.name,
    tasks,
    createdAt: new Date().toISOString(),
    community: true
  };
}

export function createImportedTemplate(templateData: TemplateData): TaskTemplate {
  return {
    id: Date.now(),
    name: templateData.name,
    tasks: templateData.tasks,
    createdAt: new Date().toISOString(),
    imported: true
  };
}

export function upsertTemplateByName(templates: TaskTemplate[], template: TaskTemplate): TaskTemplate[] {
  return [
    ...templates.filter(existingTemplate => existingTemplate.name !== template.name),
    template
  ];
}

export function removeTemplateById(templates: TaskTemplate[], templateId: number): TaskTemplate[] {
  return templates.filter(template => template.id !== templateId);
}

export function instantiateTemplateTasks(template: TaskTemplate): Task[] {
  const loadedTasks = template.tasks.map(task => createRuntimeTaskFromTemplate(task));
  remapTemplateTriggersOnLoad(template.tasks, loadedTasks);
  return loadedTasks;
}

export function createTemplateExportData(template: TaskTemplate): TemplateExportData {
  const hasBackgroundTasks = hasBackgroundTask(template.tasks);

  return {
    name: template.name,
    tasks: template.tasks,
    version: hasBackgroundTasks ? '2.0' : '1.0',
    exportedAt: new Date().toISOString(),
    description: `PoE Task Template: ${template.name}`,
    taskCount: countTemplateTasks(template.tasks)
  };
}

export function createTemplateExportFilename(templateName: string): string {
  const slug = templateName.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '') || 'template';
  return `poe2-template-${slug}.json`;
}

function remapTemplateTriggersOnLoad(templateTasks: TemplateTask[], loadedTasks: Task[]): void {
  const flatLoaded = flattenTasks(loadedTasks);
  const flatTemplate = flattenTemplateTasks(templateTasks);

  flatTemplate.forEach((templateTask, index) => {
    if (!templateTask.triggerIndices?.length || !flatLoaded[index]) {
      return;
    }

    flatLoaded[index].triggers = templateTask.triggerIndices
      .map(triggerIndex => flatLoaded[triggerIndex]?.id)
      .filter((id): id is number => id !== undefined);
  });
}

export function countTemplateTasks(taskArray: TemplateTask[]): number {
  return flattenTemplateTasks(taskArray).length;
}

function flattenTemplateTasks(taskArray: TemplateTask[]): TemplateTask[] {
  return taskArray.flatMap(task => [
    task,
    ...flattenTemplateTasks(task.children || [])
  ]);
}
