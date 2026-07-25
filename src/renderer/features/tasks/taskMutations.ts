import type {
  BackgroundPriority,
  Task
} from '../../../shared/types';
import { createTask } from './taskCreation';
import {
  collectTaskIds,
  findTaskById,
  removeTaskFromParent
} from './taskTree';

export interface AddTaskResult {
  task: Task;
  parentTask: Task | null;
}

export interface DeleteTaskResult {
  removed: boolean;
  removedTaskIds: number[];
}

export function addTaskToList(taskList: Task[], text: string, parentId: number | null = null): AddTaskResult | null {
  const parentTask = parentId !== null ? findTaskById(parentId, taskList) : null;
  if (parentId !== null && !parentTask) {
    return null;
  }

  const task = createTask(text, parentTask);
  if (parentTask) {
    parentTask.children.push(task);
  } else {
    taskList.push(task);
  }

  return {
    task,
    parentTask
  };
}

export function deleteTaskFromList(taskId: number, taskList: Task[]): DeleteTaskResult {
  const taskToDelete = findTaskById(taskId, taskList);
  const removedTaskIds = taskToDelete ? collectTaskIds(taskToDelete) : [taskId];

  removeTriggerReferences(taskList, new Set(removedTaskIds));

  return {
    removed: removeTaskFromParent(taskId, taskList),
    removedTaskIds
  };
}

export function editTaskInList(
  taskId: number,
  taskList: Task[],
  text: string,
  priority?: BackgroundPriority
): boolean {
  const normalizedText = text.trim();
  if (!normalizedText) {
    return false;
  }

  const task = findTaskById(taskId, taskList);
  if (!task) {
    return false;
  }

  const priorityChanged = task.mode === 'background' &&
    priority !== undefined &&
    priority !== (task.backgroundOptions?.priority ?? 'normal');
  if (normalizedText === task.text && !priorityChanged) {
    return false;
  }

  task.text = normalizedText;
  if (priorityChanged) {
    task.backgroundOptions = {
      expiresAfterMinutes: task.backgroundOptions?.expiresAfterMinutes ?? null,
      priority
    };
  }
  return true;
}

function removeTriggerReferences(taskList: Task[], removedTaskIds: Set<number>): void {
  taskList.forEach(task => {
    task.triggers = task.triggers.filter(id => !removedTaskIds.has(id));
    removeTriggerReferences(task.children, removedTaskIds);
  });
}
