import type {
  Task,
  TaskLocation
} from '../../../shared/types';

export function findTaskById(id: number, taskList: Task[]): Task | null {
  for (const task of taskList) {
    if (task.id === id) {
      return task;
    }
    if (task.children.length > 0) {
      const found = findTaskById(id, task.children);
      if (found) return found;
    }
  }
  return null;
}

export function flattenTasks(taskList: Task[]): Task[] {
  const flattenedTasks: Task[] = [];

  function visit(tasks: Task[]): void {
    tasks.forEach(task => {
      flattenedTasks.push(task);
      visit(task.children);
    });
  }

  visit(taskList);
  return flattenedTasks;
}

export function collectTaskIds(task: Task): number[] {
  return flattenTasks([task]).map(currentTask => currentTask.id);
}

export function findParentTask(childId: number, taskList: Task[]): Task | null {
  for (const task of taskList) {
    for (const child of task.children) {
      if (child.id === childId) {
        return task;
      }
    }
    const found = findParentTask(childId, task.children);
    if (found) return found;
  }
  return null;
}

export function findTaskWithParent(
  taskId: number,
  taskArray: Task[],
  parent: Task | null = null,
  level = 0
): TaskLocation | null {
  for (const task of taskArray) {
    if (task.id === taskId) {
      return { task, parent, level, array: taskArray };
    }

    if (task.children.length > 0) {
      const result = findTaskWithParent(taskId, task.children, task, level + 1);
      if (result) return result;
    }
  }
  return null;
}

export function removeTaskFromParent(taskId: number, taskList: Task[]): boolean {
  function removeFromArray(taskArray: Task[]): boolean {
    for (let i = 0; i < taskArray.length; i++) {
      if (taskArray[i].id === taskId) {
        taskArray.splice(i, 1);
        return true;
      }

      if (removeFromArray(taskArray[i].children)) {
        return true;
      }
    }
    return false;
  }

  return removeFromArray(taskList);
}
