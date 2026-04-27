import type { Task, TaskLocation } from '../../../shared/types';
import {
  findTaskWithParent,
  removeTaskFromParent
} from './taskTree';

export interface TaskDropOperation {
  isAbove: boolean;
  isIndented: boolean;
  makeSubtask: boolean;
  operationType: string;
  placeholderColor: string;
  draggedInfo: TaskLocation;
  targetInfo: TaskLocation;
  targetTaskId: number;
}

export interface MoveTaskResult {
  success: boolean;
  operation?: string;
  reason?: string;
}

export function getTaskDropOperation(
  taskList: Task[],
  draggedTaskId: number | null,
  targetTaskId: number,
  isAbove: boolean,
  isIndented: boolean
): TaskDropOperation | null {
  if (!draggedTaskId || draggedTaskId === targetTaskId) {
    return null;
  }

  const draggedInfo = findTaskWithParent(draggedTaskId, taskList);
  const targetInfo = findTaskWithParent(targetTaskId, taskList);
  if (!draggedInfo || !targetInfo || isDescendantTask(draggedInfo.task, targetTaskId)) {
    return null;
  }

  const makeSubtask = isIndented && !isAbove && draggedInfo.level <= targetInfo.level;
  const { operationType, placeholderColor } = describeDropOperation(makeSubtask, draggedInfo.level, targetInfo.level);

  return {
    isAbove,
    isIndented,
    makeSubtask,
    operationType,
    placeholderColor,
    draggedInfo,
    targetInfo,
    targetTaskId
  };
}

export function moveTaskInTree(
  taskList: Task[],
  draggedId: number,
  targetId: number,
  insertAbove: boolean,
  makeSubtask = false
): MoveTaskResult {
  const draggedInfo = findTaskWithParent(draggedId, taskList);
  const targetInfo = findTaskWithParent(targetId, taskList);

  if (!draggedInfo || !targetInfo) {
    return {
      success: false,
      reason: `Task not found: ${!draggedInfo ? draggedId : targetId}`
    };
  }

  if ((draggedInfo.task.mode || 'main') !== (targetInfo.task.mode || 'main')) {
    return {
      success: false,
      reason: 'Cannot reorder across main/background lists'
    };
  }

  if (isDescendantTask(draggedInfo.task, targetId)) {
    return {
      success: false,
      reason: 'Cannot move a task into its own subtree'
    };
  }

  const draggedTask = draggedInfo.task;
  const originalArray = draggedInfo.array;
  const originalIndex = originalArray.findIndex(task => task.id === draggedId);
  if (!removeTaskFromParent(draggedId, taskList)) {
    return {
      success: false,
      reason: `Could not remove task ${draggedId}`
    };
  }

  const destination = getTaskDestination(taskList, draggedInfo, targetInfo, targetId, insertAbove, makeSubtask);
  if (!destination) {
    restoreTask(originalArray, originalIndex, draggedTask);
    return {
      success: false,
      reason: `Could not find drop target ${targetId}`
    };
  }

  destination.array.splice(destination.index, 0, draggedTask);

  return {
    success: true,
    operation: createMoveDescription(draggedInfo, targetInfo, makeSubtask)
  };
}

function getTaskDestination(
  taskList: Task[],
  draggedInfo: TaskLocation,
  targetInfo: TaskLocation,
  targetId: number,
  insertAbove: boolean,
  makeSubtask: boolean
): { array: Task[]; index: number } | null {
  if (makeSubtask) {
    return {
      array: targetInfo.task.children,
      index: targetInfo.task.children.length
    };
  }

  const destinationArray = targetInfo.parent ? targetInfo.parent.children : taskList;
  const targetIndex = destinationArray.findIndex(task => task.id === targetId);
  if (targetIndex < 0) {
    return null;
  }

  return {
    array: destinationArray,
    index: insertAbove ? targetIndex : targetIndex + 1
  };
}

function createMoveDescription(draggedInfo: TaskLocation, targetInfo: TaskLocation, makeSubtask: boolean): string {
  if (makeSubtask) {
    return `Made "${draggedInfo.task.text}" a subtask of "${targetInfo.task.text}"`;
  }

  if (draggedInfo.level === targetInfo.level) {
    return `Reordered "${draggedInfo.task.text}" within same level`;
  }

  if (draggedInfo.level > targetInfo.level) {
    return `Promoted "${draggedInfo.task.text}" from level ${draggedInfo.level} to level ${targetInfo.level}`;
  }

  return `Moved "${draggedInfo.task.text}" to be sibling of "${targetInfo.task.text}"`;
}

function describeDropOperation(makeSubtask: boolean, draggedLevel: number, targetLevel: number): {
  operationType: string;
  placeholderColor: string;
} {
  if (makeSubtask) {
    return {
      operationType: 'Make subtask of target',
      placeholderColor: '#4285f4'
    };
  }

  if (draggedLevel === targetLevel) {
    return {
      operationType: 'Reorder within same level',
      placeholderColor: '#d4af37'
    };
  }

  if (draggedLevel > targetLevel) {
    return {
      operationType: 'Promote to higher level',
      placeholderColor: '#32cd32'
    };
  }

  return {
    operationType: 'Reorder as sibling',
    placeholderColor: '#d4af37'
  };
}

function isDescendantTask(parentTask: Task, taskId: number): boolean {
  return parentTask.children.some(child => child.id === taskId || isDescendantTask(child, taskId));
}

function restoreTask(originalArray: Task[], originalIndex: number, task: Task): void {
  const safeIndex = originalIndex < 0
    ? originalArray.length
    : Math.min(originalIndex, originalArray.length);

  originalArray.splice(safeIndex, 0, task);
}
