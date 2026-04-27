const VISIBLE_MODAL_CLASS = 'is-visible';

export interface ShowModalOptions {
  focusSelector?: string;
  focusDelayMs?: number;
  focusRetryDelaysMs?: readonly number[];
  focusSettleDelayMs?: number;
  focusWindow?: () => void;
  selectText?: boolean;
}

export function showModal(id: string, options: ShowModalOptions = {}): HTMLElement | null {
  const modal = document.getElementById(id);
  if (modal) {
    modal.classList.add(VISIBLE_MODAL_CLASS);
    modal.setAttribute('aria-hidden', 'false');
    scheduleModalFocus(modal, options);
  }
  return modal;
}

export function hideModal(id: string): HTMLElement | null {
  const modal = document.getElementById(id);
  if (modal) {
    modal.classList.remove(VISIBLE_MODAL_CLASS);
    modal.setAttribute('aria-hidden', 'true');
  }
  return modal;
}

function scheduleModalFocus(modal: HTMLElement, options: ShowModalOptions): void {
  if (!options.focusSelector) {
    return;
  }

  const focusDelayMs = options.focusDelayMs ?? 0;
  const focusRetryDelaysMs = options.focusRetryDelaysMs ?? [0, 75, 200, 400];
  const focusSettleDelayMs = options.focusSettleDelayMs ?? 50;

  focusRetryDelaysMs.forEach(delayMs => {
    setTimeout(() => focusContainingWindow(options.focusWindow), focusDelayMs + delayMs);
    setTimeout(() => focusModalTarget(modal, options), focusDelayMs + delayMs + focusSettleDelayMs);
  });
}

function focusModalTarget(modal: HTMLElement, options: ShowModalOptions): void {
  if (!modal.classList.contains(VISIBLE_MODAL_CLASS) || !options.focusSelector) {
    return;
  }

  const target = modal.querySelector<HTMLElement>(options.focusSelector);
  if (!target || isDisabledControl(target)) {
    return;
  }

  target.focus();

  if (options.selectText && isSelectableTextControl(target)) {
    target.select();
  }
}

function focusContainingWindow(focusWindow?: () => void): void {
  try {
    if (focusWindow) {
      focusWindow();
      return;
    }

    window.focus?.();
  } catch {
    // Some test/browser environments expose focus but do not implement it.
  }
}

function isDisabledControl(element: HTMLElement): boolean {
  return (
    element instanceof HTMLButtonElement ||
    element instanceof HTMLInputElement ||
    element instanceof HTMLSelectElement ||
    element instanceof HTMLTextAreaElement
  ) && element.disabled;
}

function isSelectableTextControl(element: HTMLElement): element is HTMLInputElement | HTMLTextAreaElement {
  return element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement;
}
