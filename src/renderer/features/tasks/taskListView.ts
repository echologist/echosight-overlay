import type { Task } from '../../../shared/types';
import {
  renderTaskItems,
  type TaskListRenderCallbacks
} from './taskListRenderer';

export function renderTaskLists(tasks: Task[], callbacks: TaskListRenderCallbacks): void {
  const taskList = document.getElementById('taskList');
  const backgroundTaskList = document.getElementById('backgroundTaskList');

  if (!taskList || !backgroundTaskList) {
    return;
  }

  taskList.replaceChildren();
  backgroundTaskList.replaceChildren();

  const mainTasks = tasks.filter(task => task.mode !== 'background');
  const activeBackgroundTasks = tasks.filter(task => task.mode === 'background' && task.activated);

  renderTaskItems(mainTasks, taskList, callbacks);
  renderTaskItems(activeBackgroundTasks, backgroundTaskList, callbacks, { isBackground: true });
  renderBackgroundSection(activeBackgroundTasks);
}

function renderBackgroundSection(activeBackgroundTasks: Task[]): void {
  const backgroundSection = document.getElementById('backgroundTasksSection');
  if (!backgroundSection) {
    return;
  }

  backgroundSection.classList.toggle('has-active-background-tasks', activeBackgroundTasks.length > 0);

  const backgroundCount = document.getElementById('backgroundCount');
  if (backgroundCount) {
    backgroundCount.textContent = activeBackgroundTasks.length > 0
      ? `(${activeBackgroundTasks.filter(task => task.completed).length}/${activeBackgroundTasks.length})`
      : '';
  }
}
