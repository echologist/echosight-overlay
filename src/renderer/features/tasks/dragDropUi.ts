import type { TaskDropOperation } from './taskTreeOperations';

export function parseTaskIdFromElement(element: HTMLElement | null): number | null {
  if (!element?.dataset.taskId) {
    return null;
  }

  const taskId = Number.parseFloat(element.dataset.taskId);
  return Number.isFinite(taskId) ? taskId : null;
}

export function getDropIntent(event: DragEvent, targetElement: HTMLElement): {
  isAbove: boolean;
  isIndented: boolean;
} {
  const rect = targetElement.getBoundingClientRect();
  const midpoint = rect.top + rect.height / 2;
  const rightHalf = rect.left + rect.width * 0.5;

  return {
    isAbove: event.clientY < midpoint,
    isIndented: event.clientX > rightHalf
  };
}

export function createInitialDragPlaceholder(height: number): HTMLLIElement {
  const placeholder = document.createElement('li');
  placeholder.className = 'drag-placeholder drag-placeholder-initial';
  placeholder.style.setProperty('--drag-placeholder-height', `${height}px`);
  placeholder.textContent = 'Drop here to reorder';
  return placeholder;
}

export function renderDragPlaceholder(
  operation: TaskDropOperation,
  targetElement: HTMLElement,
  existingPlaceholder: HTMLLIElement | null
): HTMLLIElement {
  removeDragPlaceholder(existingPlaceholder);

  const placeholder = document.createElement('li');
  placeholder.className = 'drag-placeholder';
  placeholder.style.setProperty('--drag-placeholder-height', `${targetElement.offsetHeight}px`);

  applyPlaceholderOperationStyle(placeholder, operation);
  insertPlaceholder(placeholder, operation, targetElement);

  return placeholder;
}

export function removeDragPlaceholder(placeholder: HTMLLIElement | null): void {
  if (placeholder?.parentNode) {
    placeholder.parentNode.removeChild(placeholder);
  }
}

export function getDragOperationKey(operation: TaskDropOperation): string {
  return `${operation.operationType}-${operation.isAbove}-${operation.targetTaskId}`;
}

export function findTaskDropTarget(
  eventTarget: EventTarget | null,
  clientY: number,
  maxDistance = Infinity,
  root: ParentNode = document
): HTMLElement | null {
  const target = getClosestTaskItem(eventTarget);
  if (target) {
    return target;
  }

  const placeholderTarget = getAdjacentTaskForPlaceholder(eventTarget);
  if (placeholderTarget) {
    return placeholderTarget;
  }

  return findNearestTaskItem(clientY, maxDistance, root);
}

export function setDragStartStyle(element: HTMLElement): void {
  element.classList.add('is-dragging');
}

export function resetDragStyle(element: HTMLElement): void {
  element.classList.remove('is-dragging');
}

export function setDragHoverStyle(element: HTMLElement): void {
  element.classList.add('is-drag-hover');
}

export function resetDragHoverStyle(element: HTMLElement): void {
  element.classList.remove('is-drag-hover');
}

function applyPlaceholderOperationStyle(placeholder: HTMLLIElement, operation: TaskDropOperation): void {
  if (operation.makeSubtask) {
    placeholder.classList.add('drag-placeholder-subtask');
    placeholder.style.setProperty('--drag-placeholder-color', '#4285f4');
    placeholder.style.setProperty('--drag-placeholder-indent', `${(operation.targetInfo.level + 1) * 20}px`);
    placeholder.textContent = 'Drop to make subtask';
    return;
  }

  if (operation.operationType.includes('Promote')) {
    placeholder.classList.add('drag-placeholder-promote');
    placeholder.style.setProperty('--drag-placeholder-color', '#32cd32');
    placeholder.textContent = operation.operationType;
    return;
  }

  placeholder.style.setProperty('--drag-placeholder-color', operation.placeholderColor);
  placeholder.textContent = operation.operationType;
}

function insertPlaceholder(
  placeholder: HTMLLIElement,
  operation: TaskDropOperation,
  targetElement: HTMLElement
): void {
  if (!targetElement.parentNode) {
    return;
  }

  if (operation.makeSubtask) {
    targetElement.parentNode.insertBefore(placeholder, targetElement.nextSibling);
    return;
  }

  targetElement.parentNode.insertBefore(
    placeholder,
    operation.isAbove ? targetElement : targetElement.nextSibling
  );
}

function getClosestTaskItem(eventTarget: EventTarget | null): HTMLElement | null {
  if (!(eventTarget instanceof Element)) {
    return null;
  }

  const taskItem = eventTarget.closest('.task-item');
  return taskItem instanceof HTMLElement ? taskItem : null;
}

function getAdjacentTaskForPlaceholder(eventTarget: EventTarget | null): HTMLElement | null {
  if (!(eventTarget instanceof Element) || !eventTarget.classList.contains('drag-placeholder')) {
    return null;
  }

  const previous = eventTarget.previousElementSibling;
  if (isTaskItem(previous)) {
    return previous;
  }

  const next = eventTarget.nextElementSibling;
  return isTaskItem(next) ? next : null;
}

function findNearestTaskItem(clientY: number, maxDistance: number, root: ParentNode): HTMLElement | null {
  const taskItems = Array.from(root.querySelectorAll<HTMLElement>('.task-item'));
  let closestTask: HTMLElement | null = null;
  let closestDistance = maxDistance;

  taskItems.forEach(taskItem => {
    const rect = taskItem.getBoundingClientRect();
    const distance = Math.abs(clientY - (rect.top + rect.height / 2));
    if (distance < closestDistance) {
      closestDistance = distance;
      closestTask = taskItem;
    }
  });

  return closestTask;
}

function isTaskItem(element: Element | null): element is HTMLElement {
  return element instanceof HTMLElement && element.classList.contains('task-item');
}
