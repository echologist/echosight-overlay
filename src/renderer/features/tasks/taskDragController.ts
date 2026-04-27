import type { Task } from '../../../shared/types';
import {
  createInitialDragPlaceholder,
  findTaskDropTarget,
  getDragOperationKey,
  getDropIntent,
  parseTaskIdFromElement,
  removeDragPlaceholder,
  renderDragPlaceholder,
  resetDragHoverStyle,
  resetDragStyle,
  setDragHoverStyle,
  setDragStartStyle
} from './dragDropUi';
import {
  getTaskDropOperation,
  type TaskDropOperation
} from './taskTreeOperations';

type LogSink = Pick<Console, 'log'>;
type TaskListElement = HTMLElement & { hasDropHandler?: boolean };

export interface TaskDragControllerOptions {
  getTasks: () => Task[];
  onReorder: (draggedId: number, targetId: number, insertAbove: boolean, makeSubtask: boolean) => void;
  logger?: LogSink;
}

export interface TaskDragController {
  initializeDragAndDrop: () => void;
  setupContainerDropHandlers: () => void;
}

export function createTaskDragController(options: TaskDragControllerOptions): TaskDragController {
  const logger = options.logger || console;
  let draggedTaskId: number | null = null;
  let draggedElement: HTMLElement | null = null;
  let placeholder: HTMLLIElement | null = null;
  let lastOperation: string | null = null;

  function initializeDragAndDrop(): void {
    document.querySelectorAll<HTMLElement>('.task-item').forEach(taskItem => {
      taskItem.draggable = true;

      taskItem.addEventListener('dragstart', handleDragStart);
      taskItem.addEventListener('dragend', handleDragEnd);
      taskItem.addEventListener('dragover', handleDragOver);
      taskItem.addEventListener('drop', handleDrop);
      taskItem.addEventListener('dragenter', handleDragEnter);
      taskItem.addEventListener('dragleave', handleDragLeave);
    });
  }

  function handleDragStart(event: DragEvent): void {
    const currentTarget = getCurrentTaskElement(event);
    if (!currentTarget) {
      return;
    }

    draggedTaskId = parseTaskIdFromElement(currentTarget);
    draggedElement = currentTarget;

    setDragStartStyle(currentTarget);
    placeholder = createInitialDragPlaceholder(currentTarget.offsetHeight);

    if (event.dataTransfer) {
      event.dataTransfer.effectAllowed = 'move';
      event.dataTransfer.setData('text/html', currentTarget.outerHTML);
    }
  }

  function handleDragEnd(): void {
    if (draggedElement) {
      resetDragStyle(draggedElement);
    }

    removeDragPlaceholder(placeholder);
    draggedTaskId = null;
    draggedElement = null;
    placeholder = null;
    lastOperation = null;
  }

  function handleDragOver(event: DragEvent): void {
    event.preventDefault();
    event.stopPropagation();
    if (event.dataTransfer) {
      event.dataTransfer.dropEffect = 'move';
    }

    const targetElement = getCurrentTaskElement(event);
    if (!targetElement || targetElement === draggedElement) {
      return;
    }

    const operation = calculateDropOperation(event, targetElement);
    if (!operation) {
      return;
    }

    renderPlaceholderIfChanged(operation, targetElement);
  }

  function handleDrop(event: DragEvent): void {
    event.preventDefault();
    logger.log('Drop triggered');

    const targetElement = getCurrentTaskElement(event);
    if (!draggedTaskId || !targetElement) {
      return;
    }

    const targetTaskId = parseTaskIdFromElement(targetElement);
    if (targetTaskId === null) {
      return;
    }

    const operation = calculateDropOperation(event, targetElement);
    if (operation) {
      options.onReorder(draggedTaskId, targetTaskId, operation.isAbove, operation.makeSubtask);
    }
  }

  function handleDragEnter(event: DragEvent): void {
    event.preventDefault();
    event.stopPropagation();

    const currentTarget = getCurrentTaskElement(event);
    if (currentTarget && currentTarget !== draggedElement) {
      setDragHoverStyle(currentTarget);
    }
  }

  function handleDragLeave(event: DragEvent): void {
    const currentTarget = getCurrentTaskElement(event);
    if (currentTarget && currentTarget !== draggedElement) {
      resetDragHoverStyle(currentTarget);
    }
  }

  function setupContainerDropHandlers(): void {
    setupListDropHandler('taskList', Infinity, true);
    setupListDropHandler('backgroundTaskList', Infinity, false);
  }

  function setupListDropHandler(listId: string, dropMaxDistance: number, verbose: boolean): void {
    const taskList = getTaskListElement(listId);
    if (!taskList || taskList.hasDropHandler) {
      return;
    }

    taskList.addEventListener('drop', event => {
      event.preventDefault();
      if (verbose) {
        logger.log('Container drop event fired!');
        logger.log('Drop target element:', event.target);
        logger.log('Dragged task ID:', draggedTaskId);
      }

      const target = findTaskDropTarget(event.target, event.clientY, dropMaxDistance, taskList);
      handleContainerDrop(event, target, verbose);
    });

    taskList.addEventListener('dragover', event => {
      event.preventDefault();
      const target = findTaskDropTarget(event.target, event.clientY, 50, taskList);
      if (target && draggedTaskId) {
        const operation = calculateDropOperation(event, target);
        if (operation) {
          renderPlaceholderIfChanged(operation, target);
        }
      }
    });

    taskList.hasDropHandler = true;
  }

  function handleContainerDrop(event: DragEvent, target: HTMLElement | null, verbose: boolean): void {
    const targetTaskId = parseTaskIdFromElement(target);
    if (verbose) {
      logger.log('Found task item target:', target);
      logger.log('Target dataset:', target ? target.dataset : 'no target');
    }

    if (target && targetTaskId !== null && draggedTaskId) {
      if (draggedTaskId === targetTaskId) {
        return;
      }

      const operation = calculateDropOperation(event, target);
      if (operation) {
        if (verbose) {
          logger.log('Drop operation details:', {
            isAbove: operation.isAbove,
            isIndented: operation.isIndented,
            makeSubtask: operation.makeSubtask,
            operationType: operation.operationType,
            draggedLevel: operation.draggedInfo.level,
            targetLevel: operation.targetInfo.level
          });
        }
        options.onReorder(draggedTaskId, operation.targetTaskId, operation.isAbove, operation.makeSubtask);
      }
      return;
    }

    if (verbose) {
      logger.log('Drop validation failed:', {
        hasTarget: !!target,
        hasTaskId: targetTaskId !== null,
        hasDraggedId: !!draggedTaskId
      });
    }
  }

  function calculateDropOperation(event: DragEvent, targetElement: HTMLElement): TaskDropOperation | null {
    if (!draggedTaskId) {
      return null;
    }

    const targetTaskId = parseTaskIdFromElement(targetElement);
    if (targetTaskId === null) {
      return null;
    }

    const { isAbove, isIndented } = getDropIntent(event, targetElement);
    return getTaskDropOperation(options.getTasks(), draggedTaskId, targetTaskId, isAbove, isIndented);
  }

  function renderPlaceholderIfChanged(operation: TaskDropOperation, targetElement: HTMLElement): void {
    const operationKey = getDragOperationKey(operation);
    if (lastOperation === operationKey) {
      return;
    }

    lastOperation = operationKey;
    placeholder = renderDragPlaceholder(operation, targetElement, placeholder);
  }

  return {
    initializeDragAndDrop,
    setupContainerDropHandlers
  };
}

function getCurrentTaskElement(event: DragEvent): HTMLElement | null {
  return event.currentTarget instanceof HTMLElement ? event.currentTarget : null;
}

function getTaskListElement(id: string): TaskListElement | null {
  const element = document.getElementById(id);
  return element instanceof HTMLElement ? element : null;
}
