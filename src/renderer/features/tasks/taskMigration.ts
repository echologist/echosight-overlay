import type { Task } from '../../../shared/types';
import { isRecord } from './taskGuards';

type TaskRecord = Partial<Omit<Task, 'children' | 'triggers'>> & {
  children?: unknown;
  triggers?: unknown;
};

export function migrateTaskStructure(taskList: unknown): boolean {
  let migrated = false;

  function migrateTask(task: TaskRecord): void {
    if (!Array.isArray(task.children)) { task.children = []; migrated = true; }
    if (!task.mode) { task.mode = 'main'; migrated = true; }
    if (!Array.isArray(task.triggers)) { task.triggers = []; migrated = true; }
    if (task.activated === undefined) { task.activated = true; migrated = true; }
    if (task.activatedAt === undefined) { task.activatedAt = null; migrated = true; }
    if (task.backgroundOptions === undefined) { task.backgroundOptions = null; migrated = true; }
    const children = task.children;
    if (!Array.isArray(children)) return;

    children
      .filter(isRecord)
      .forEach(child => migrateTask(child as TaskRecord));
  }

  if (!Array.isArray(taskList)) {
    return false;
  }

  taskList
    .filter(isRecord)
    .forEach(task => migrateTask(task as TaskRecord));
  return migrated;
}
