import type { Task } from '../../../shared/types';

export interface TaskListRenderCallbacks {
  onToggleTask: (taskId: number) => void;
  onContextMenu: (event: MouseEvent, taskId: number, isParent: boolean) => void;
  onConfigureTriggers: (taskId: number) => void;
  onDeleteTask: (taskId: number) => void | Promise<void>;
}

export interface TaskListRenderOptions {
  isBackground?: boolean;
  level?: number;
}

export function renderTaskItems(
  taskArray: Task[],
  targetList: HTMLElement,
  callbacks: TaskListRenderCallbacks,
  options: TaskListRenderOptions = {}
): void {
  const level = options.level || 0;
  const isBackground = !!options.isBackground;

  taskArray.forEach(task => {
    targetList.appendChild(createTaskItem(task, level, isBackground, callbacks));

    if (task.children.length > 0) {
      renderTaskItems(task.children, targetList, callbacks, {
        isBackground,
        level: level + 1
      });
    }
  });
}

function createTaskItem(
  task: Task,
  level: number,
  isBackground: boolean,
  callbacks: TaskListRenderCallbacks
): HTMLLIElement {
  const li = document.createElement('li');
  li.className = 'task-item';
  li.dataset.taskId = String(task.id);
  li.dataset.level = String(level);
  li.dataset.mode = task.mode || 'main';
  li.draggable = true;

  if (isBackground && task.backgroundOptions?.priority === 'high') {
    li.classList.add('high-priority');
  }

  li.appendChild(createTaskRow(task, level, callbacks));
  return li;
}

function createTaskRow(task: Task, level: number, callbacks: TaskListRenderCallbacks): HTMLDivElement {
  const row = document.createElement('div');
  row.className = 'task-row';
  row.style.setProperty('--task-row-indent', `${level * 20}px`);

  const isParent = task.children.length > 0;
  row.appendChild(createDragHandle());
  row.appendChild(createParentIcon(isParent));
  row.appendChild(createCheckbox(task, callbacks));
  row.appendChild(createTaskText(task, isParent, callbacks));
  row.appendChild(createTaskBadges(task, isParent, callbacks));
  row.appendChild(createRightClickHint());
  row.appendChild(createDeleteButton(task, callbacks));

  return row;
}

function createDragHandle(): HTMLSpanElement {
  const dragHandle = document.createElement('span');
  dragHandle.className = 'drag-handle';
  dragHandle.title = 'Drag to reorder';
  dragHandle.textContent = '⋮⋮';
  return dragHandle;
}

function createParentIcon(isParent: boolean): HTMLSpanElement {
  const parentIcon = document.createElement('span');
  if (isParent) {
    parentIcon.className = 'parent-icon';
    parentIcon.textContent = '📁';
  } else {
    parentIcon.className = 'parent-icon-spacer';
  }
  return parentIcon;
}

function createCheckbox(task: Task, callbacks: TaskListRenderCallbacks): HTMLInputElement {
  const checkbox = document.createElement('input');
  checkbox.type = 'checkbox';
  checkbox.className = 'task-checkbox';
  checkbox.checked = !!task.completed;
  checkbox.addEventListener('change', () => callbacks.onToggleTask(task.id));
  return checkbox;
}

function createTaskText(
  task: Task,
  isParent: boolean,
  callbacks: TaskListRenderCallbacks
): HTMLSpanElement {
  const taskText = document.createElement('span');
  taskText.className = `task-text ${task.completed ? 'completed' : ''}`;
  if (isParent) {
    taskText.classList.add('parent-task-text');
  }
  taskText.textContent = task.text;
  taskText.addEventListener('contextmenu', event => {
    callbacks.onContextMenu(event, task.id, isParent);
  });
  return taskText;
}

function createTaskBadges(
  task: Task,
  isParent: boolean,
  callbacks: TaskListRenderCallbacks
): HTMLSpanElement {
  const badges = document.createElement('span');
  badges.className = 'task-badges';

  if (task.triggers.length > 0) {
    const triggerIndicator = document.createElement('span');
    triggerIndicator.className = 'trigger-indicator';
    triggerIndicator.title = `Triggers ${task.triggers.length} background task(s)`;
    triggerIndicator.textContent = `⚡${task.triggers.length}`;
    triggerIndicator.addEventListener('click', () => callbacks.onConfigureTriggers(task.id));
    badges.appendChild(triggerIndicator);
  }

  if (isParent) {
    const childCount = document.createElement('span');
    childCount.className = 'child-count';
    childCount.textContent = `(${task.children.filter(child => child.completed).length}/${task.children.length})`;
    badges.appendChild(childCount);
  }

  return badges;
}

function createRightClickHint(): HTMLSpanElement {
  const rightClickHint = document.createElement('span');
  rightClickHint.className = 'right-click-hint';
  rightClickHint.textContent = 'Right-click for options';
  return rightClickHint;
}

function createDeleteButton(task: Task, callbacks: TaskListRenderCallbacks): HTMLButtonElement {
  const deleteButton = document.createElement('button');
  deleteButton.className = 'task-delete';
  deleteButton.textContent = '×';
  deleteButton.addEventListener('click', () => {
    void callbacks.onDeleteTask(task.id);
  });
  return deleteButton;
}
