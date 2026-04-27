import type { Task } from '../../../shared/types';
import { hideModal, showModal } from '../../ui/modalUi';
import {
  focusOverlayNow,
  type FocusOverlayApi
} from '../../ui/windowFocus';

const TRIGGER_CHECKBOX_CLASS = 'trigger-checkbox';

export function showBackgroundTaskNotification(count: number): void {
  const notification = document.createElement('div');
  notification.className = 'bg-task-notification';
  notification.textContent = `${count} background task${count > 1 ? 's' : ''} activated!`;
  document.body.appendChild(notification);

  setTimeout(() => {
    if (notification.parentNode) {
      notification.classList.add('toast-exit');
      setTimeout(() => notification.remove(), 300);
    }
  }, 3000);
}

export function renderTriggerTaskList(
  container: HTMLElement,
  task: Task,
  backgroundTasks: Task[]
): void {
  container.replaceChildren();

  if (backgroundTasks.length === 0) {
    container.appendChild(createEmptyTriggerMessage());
    return;
  }

  backgroundTasks.forEach(backgroundTask => {
    container.appendChild(createTriggerTaskRow(task, backgroundTask));
  });
}

export function showTriggerConfigModal(
  task: Task,
  backgroundTasks: Task[],
  api?: FocusOverlayApi
): boolean {
  const modal = document.getElementById('configureTriggersModal');
  const listContainer = document.getElementById('triggerTaskList');
  if (!modal || !listContainer) {
    return false;
  }

  renderTriggerTaskList(listContainer, task, backgroundTasks);
  clearNewBackgroundTaskInput();
  showModal('configureTriggersModal', {
    focusSelector: '#newBgTaskInput',
    focusWindow: () => focusOverlayNow(api),
    selectText: true
  });
  return true;
}

export function closeTriggerConfigModal(): void {
  hideModal('configureTriggersModal');
}

export function readNewBackgroundTaskInput(): {
  text: string;
  highPriority: boolean;
} {
  return {
    text: readTrimmedInputValue(document.getElementById('newBgTaskInput')),
    highPriority: readCheckboxChecked(document.getElementById('bgTaskHighPriority'))
  };
}

function clearNewBackgroundTaskInput(): void {
  clearTextInput(document.getElementById('newBgTaskInput'));
}

export function readSelectedTriggerIds(root: ParentNode = document): number[] {
  return Array.from(root.querySelectorAll<HTMLInputElement>(`input.${TRIGGER_CHECKBOX_CLASS}`))
    .filter(checkbox => checkbox.checked)
    .map(checkbox => parseFiniteNumber(checkbox.dataset.bgTaskId))
    .filter((taskId): taskId is number => taskId !== null);
}

export function readTrimmedInputValue(element: HTMLElement | null): string {
  if (!isTextInput(element)) {
    return '';
  }

  return element.value.trim();
}

export function clearTextInput(element: HTMLElement | null): void {
  if (isTextInput(element)) {
    element.value = '';
  }
}

export function readCheckboxChecked(element: HTMLElement | null): boolean {
  return element instanceof HTMLInputElement && element.checked;
}

function createEmptyTriggerMessage(): HTMLParagraphElement {
  const emptyMessage = document.createElement('p');
  emptyMessage.className = 'trigger-empty-message';
  emptyMessage.textContent = 'No background tasks yet. Create one below.';
  return emptyMessage;
}

function createTriggerTaskRow(task: Task, backgroundTask: Task): HTMLDivElement {
  const row = document.createElement('div');
  row.className = 'trigger-task-row';

  row.appendChild(createTriggerCheckbox(task, backgroundTask));
  row.appendChild(createTriggerTaskText(backgroundTask));

  if (backgroundTask.backgroundOptions?.priority === 'high') {
    row.appendChild(createBadge('HIGH', 'trigger-badge-high'));
  }

  row.appendChild(createBadge(
    backgroundTask.activated ? 'ACTIVE' : 'DORMANT',
    backgroundTask.activated ? 'trigger-badge-active' : 'trigger-badge-dormant'
  ));

  return row;
}

function createTriggerCheckbox(task: Task, backgroundTask: Task): HTMLInputElement {
  const checkbox = document.createElement('input');
  checkbox.type = 'checkbox';
  checkbox.checked = task.triggers.includes(backgroundTask.id);
  checkbox.dataset.bgTaskId = String(backgroundTask.id);
  checkbox.className = TRIGGER_CHECKBOX_CLASS;
  return checkbox;
}

function createTriggerTaskText(backgroundTask: Task): HTMLSpanElement {
  const text = document.createElement('span');
  text.className = 'trigger-task-text';
  text.textContent = backgroundTask.text;
  return text;
}

function createBadge(label: string, modifierClass: string): HTMLSpanElement {
  const badge = document.createElement('span');
  badge.className = `trigger-badge ${modifierClass}`;
  badge.textContent = label;
  return badge;
}

function isTextInput(element: HTMLElement | null): element is HTMLInputElement | HTMLTextAreaElement {
  return element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement;
}

function parseFiniteNumber(value: string | undefined): number | null {
  if (!value) {
    return null;
  }

  const parsedValue = Number.parseFloat(value);
  return Number.isFinite(parsedValue) ? parsedValue : null;
}
