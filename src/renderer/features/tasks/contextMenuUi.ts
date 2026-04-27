export interface TaskContextMenuActions {
  onAddSubTask: (taskId: number) => void;
  onConfigureTriggers: (taskId: number) => void;
}

let activeContextMenu: HTMLDivElement | null = null;

export function showTaskContextMenu(
  event: MouseEvent,
  taskId: number,
  actions: TaskContextMenuActions
): void {
  event.preventDefault();
  closeTaskContextMenu();

  activeContextMenu = document.createElement('div');
  activeContextMenu.className = 'context-menu';

  activeContextMenu.appendChild(createMenuItem('Add sub-task', () => {
    closeTaskContextMenu();
    actions.onAddSubTask(taskId);
  }));

  const triggerItem = createMenuItem('Configure Triggers', () => {
    closeTaskContextMenu();
    actions.onConfigureTriggers(taskId);
  });
  activeContextMenu.appendChild(triggerItem);

  document.body.appendChild(activeContextMenu);
  activeContextMenu.style.left = `${event.pageX}px`;
  activeContextMenu.style.top = `${event.pageY}px`;

  setTimeout(() => {
    document.addEventListener('click', closeOnOutsideClick, { once: true });
  }, 10);
}

export function closeTaskContextMenu(): void {
  if (activeContextMenu) {
    activeContextMenu.remove();
    activeContextMenu = null;
  }
}

function createMenuItem(label: string, onClick: () => void): HTMLDivElement {
  const item = document.createElement('div');
  item.textContent = label;
  item.addEventListener('click', onClick);
  return item;
}

function closeOnOutsideClick(): void {
  closeTaskContextMenu();
}
