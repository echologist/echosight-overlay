export function showThemesPathNotification(themesPath: string): void {
  const notification = document.createElement('div');
  notification.className = 'themes-path-notification';

  const title = document.createElement('strong');
  title.textContent = 'Custom Themes Folder:';

  const pathText = document.createElement('code');
  pathText.textContent = themesPath;

  const helpText = document.createElement('small');
  helpText.textContent = 'Add .json theme files here and click "Reload" in settings';

  notification.appendChild(title);
  notification.appendChild(pathText);
  notification.appendChild(helpText);
  document.body.appendChild(notification);

  setTimeout(() => {
    if (notification.parentNode) {
      notification.remove();
    }
  }, 8000);
}

export function showReorderFeedback(message = 'Task order updated!'): void {
  const feedback = document.createElement('div');
  feedback.className = 'reorder-feedback';
  feedback.textContent = message;

  document.body.appendChild(feedback);

  setTimeout(() => {
    if (feedback.parentNode) {
      feedback.classList.add('toast-exit');
      setTimeout(() => feedback.remove(), 300);
    }
  }, 3000);
}
