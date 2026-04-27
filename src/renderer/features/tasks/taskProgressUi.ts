import type { TaskProgress } from './taskProgress';

let completedFeedbackTimer: ReturnType<typeof setTimeout> | null = null;

export function renderTaskProgress(progress: TaskProgress): void {
  clearCompletedFeedbackTimer();

  const progressFill = document.getElementById('progressFill');
  const progressText = document.getElementById('progressText');

  if (progressFill) {
    progressFill.style.width = `${progress.percentage}%`;
  }

  if (progressText) {
    progressText.textContent = `${progress.completed} / ${progress.total} tasks completed`;
    progressText.classList.remove('completed-feedback');
  }
}

export function showCompletedTaskFeedback(taskText: string): void {
  const progressText = document.getElementById('progressText');
  if (!progressText) {
    return;
  }

  clearCompletedFeedbackTimer();

  const originalText = progressText.textContent || '';
  progressText.textContent = `✓ Completed: ${taskText}`;
  progressText.classList.add('completed-feedback');

  completedFeedbackTimer = setTimeout(() => {
    progressText.textContent = originalText;
    progressText.classList.remove('completed-feedback');
    completedFeedbackTimer = null;
  }, 2000);
}

function clearCompletedFeedbackTimer(): void {
  if (completedFeedbackTimer) {
    clearTimeout(completedFeedbackTimer);
    completedFeedbackTimer = null;
  }
}
