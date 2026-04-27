import type { Task } from '../../../shared/types';
import {
  findParentTask,
  findTaskById,
  flattenTasks
} from './taskTree';

export interface CompleteTaskResult {
  task: Task;
  completedParents: Task[];
}

export interface ToggleTaskResult {
  task: Task;
  completedParents: Task[];
  uncheckedParents: Task[];
}

export function hasIncompleteTasks(taskList: Task[]): boolean {
  return flattenTasks(taskList).some(task => !task.completed);
}

export function completeAllChildren(parentTask: Task): void {
  parentTask.children.forEach(child => {
    child.completed = true;
    completeAllChildren(child);
  });
}

export function checkParentCompletion(
  childId: number,
  taskList: Task[],
  completedParents: Task[] = []
): Task[] {
  const parentTask = findParentTask(childId, taskList);
  if (parentTask && parentTask.children.length > 0) {
    const allChildrenComplete = parentTask.children.every(child => child.completed);
    if (allChildrenComplete && !parentTask.completed) {
      parentTask.completed = true;
      completedParents.push(parentTask);
      checkParentCompletion(parentTask.id, taskList, completedParents);
    }
  }
  return completedParents;
}

export function uncheckParent(
  childId: number,
  taskList: Task[],
  uncheckedParents: Task[] = []
): Task[] {
  const parentTask = findParentTask(childId, taskList);
  if (parentTask && parentTask.completed) {
    parentTask.completed = false;
    uncheckedParents.push(parentTask);
    uncheckParent(parentTask.id, taskList, uncheckedParents);
  }
  return uncheckedParents;
}

export function completeNextTaskInList(taskList: Task[]): CompleteTaskResult | null {
  const nextTask = findNextIncompleteLeafTask(taskList.filter(task => task.mode !== 'background'));
  if (!nextTask) {
    return null;
  }

  nextTask.completed = true;

  return {
    task: nextTask,
    completedParents: checkParentCompletion(nextTask.id, taskList)
  };
}

export function toggleTaskInList(taskId: number, taskList: Task[]): ToggleTaskResult | null {
  const task = findTaskById(taskId, taskList);
  if (!task) {
    return null;
  }

  task.completed = !task.completed;

  if (task.completed && task.children.length > 0) {
    completeAllChildren(task);
  }

  const completedParents = task.completed
    ? checkParentCompletion(taskId, taskList)
    : [];
  const uncheckedParents = task.completed
    ? []
    : uncheckParent(taskId, taskList);

  return {
    task,
    completedParents,
    uncheckedParents
  };
}

export function findNextIncompleteLeafTask(taskList: Task[]): Task | null {
  for (const task of taskList) {
    if (task.completed) {
      continue;
    }

    if (task.children.length > 0) {
      const leafTask = findNextIncompleteLeafTask(task.children);
      if (leafTask) {
        return leafTask;
      }
      continue;
    }

    if (task.mode !== 'background') {
      return task;
    }
  }

  return null;
}
