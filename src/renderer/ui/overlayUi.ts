import { formatHotkeyForDisplay } from './hotkeyDisplay';

type LogSink = Pick<Console, 'log' | 'error'>;

export function renderInteractiveMode(
  interactive: boolean,
  themeId: string,
  toggleHotkey: string
): void {
  const container = document.getElementById('overlayContainer');
  const hint = document.getElementById('shortcut-hint');
  const toggleButton = document.getElementById('interactiveToggle');
  const displayHotkey = formatHotkeyForDisplay(toggleHotkey);
  document.documentElement.style.setProperty(
    '--click-through-shortcut-hint',
    JSON.stringify(`${displayHotkey} to edit`)
  );

  if (!container) {
    return;
  }

  if (interactive) {
    container.classList.add('interactive');
    container.classList.remove('click-through');
    container.classList.toggle('glass-mode', themeId === 'glass');

    if (hint) {
      hint.textContent = `(Interactive Mode - ${displayHotkey} to exit)`;
    }
    if (toggleButton) {
      toggleButton.textContent = 'Interactive Mode';
      toggleButton.classList.add('interactive-toggle-active');
      toggleButton.classList.remove('interactive-toggle-inactive');
    }
    return;
  }

  container.classList.remove('interactive');
  container.classList.remove('glass-mode');
  container.classList.add('click-through');

  if (toggleButton) {
    toggleButton.textContent = 'Click-Through Mode';
    toggleButton.classList.add('interactive-toggle-inactive');
    toggleButton.classList.remove('interactive-toggle-active');
  }
}

export function setupHeaderWindowDrag(): void {
  const header = document.getElementById('header');
  if (!header) {
    return;
  }

  header.style.setProperty('-webkit-app-region', 'drag', 'important');
  header.querySelectorAll<HTMLElement>('button, input, select, textarea, a, [role="button"]').forEach(element => {
    element.style.setProperty('-webkit-app-region', 'no-drag', 'important');
  });
}

export function setupGlobalErrorLogging(logger: LogSink = console): void {
  window.addEventListener('error', event => {
    logger.error('Global error:', event.error);
    logger.error('Error message:', event.message);
    logger.error('Error location:', event.filename, event.lineno, event.colno);
  });

  window.addEventListener('unhandledrejection', event => {
    logger.error('Unhandled promise rejection:', event.reason);
  });
}
