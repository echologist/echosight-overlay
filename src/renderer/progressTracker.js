export function createProgressTracker(ctx) {
  const { state } = ctx;
  const { tasks } = state;

function updateProgress() {
  function countTasks(taskArray) {
    let completed = 0;
    let total = 0;

    taskArray.forEach(task => {
      // Count children if they exist, otherwise count the task itself
      if (task.children && task.children.length > 0) {
        const childCounts = countTasks(task.children);
        completed += childCounts.completed;
        total += childCounts.total;

        // Also count the parent task
        if (task.completed) completed++;
        total++;
      } else {
        // Leaf task
        if (task.completed) completed++;
        total++;
      }
    });

    return { completed, total };
  }

  // Count main tasks and activated background tasks separately
  const mainTasks = tasks.filter(t => t.mode !== 'background');
  const activeBgTasks = tasks.filter(t => t.mode === 'background' && t.activated);

  const mainCounts = countTasks(mainTasks);
  const bgCounts = countTasks(activeBgTasks);

  const totalCompleted = mainCounts.completed + bgCounts.completed;
  const totalAll = mainCounts.total + bgCounts.total;
  const percentage = totalAll > 0 ? (totalCompleted / totalAll) * 100 : 0;

  document.getElementById('progressFill').style.width = percentage + '%';
  document.getElementById('progressText').textContent = `${totalCompleted} / ${totalAll} tasks completed`;
}


  return {
    updateProgress
  };
}
