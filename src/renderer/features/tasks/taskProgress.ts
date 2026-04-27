import type { Task } from '../../../shared/types';

export interface TaskProgress {
  completed: number;
  total: number;
  percentage: number;
}

export function calculateTaskProgress(taskList: Task[]): TaskProgress {
  const mainCounts = countTasks(taskList.filter(task => task.mode !== 'background'));
  const backgroundCounts = countTasks(taskList.filter(task => task.mode === 'background' && task.activated));

  const completed = mainCounts.completed + backgroundCounts.completed;
  const total = mainCounts.total + backgroundCounts.total;

  return {
    completed,
    total,
    percentage: total > 0 ? (completed / total) * 100 : 0
  };
}

function countTasks(taskArray: Task[]): Pick<TaskProgress, 'completed' | 'total'> {
  let completed = 0;
  let total = 0;

  taskArray.forEach(task => {
    if (task.children.length > 0) {
      const childCounts = countTasks(task.children);
      completed += childCounts.completed;
      total += childCounts.total;
    }

    if (task.completed) {
      completed++;
    }
    total++;
  });

  return { completed, total };
}
