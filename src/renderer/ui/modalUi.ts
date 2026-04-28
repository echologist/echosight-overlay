const VISIBLE_MODAL_CLASS = 'is-visible';
const FOCUSABLE_SELECTOR = [
  'a[href]',
  'area[href]',
  'button',
  'input:not([type="hidden"])',
  'select',
  'textarea',
  'iframe',
  'object',
  'embed',
  '[contenteditable="true"]',
  '[tabindex]'
].join(',');

export interface ShowModalOptions {
  focusSelector?: string;
  focusDelayMs?: number;
  focusFirst?: boolean;
  focusRetryDelaysMs?: readonly number[];
  focusSettleDelayMs?: number;
  focusWindow?: () => void;
  restoreFocus?: boolean;
  selectText?: boolean;
  trapFocus?: boolean;
}

interface ModalFocusState {
  modal: HTMLElement;
  previousFocus: Element | null;
  restoreFocus: boolean;
  trapFocus: boolean;
  handleKeydown: (event: KeyboardEvent) => void;
}

const activeModalStack: ModalFocusState[] = [];
const modalFocusStates = new WeakMap<HTMLElement, ModalFocusState>();

export function showModal(id: string, options: ShowModalOptions = {}): HTMLElement | null {
  const modal = document.getElementById(id);
  if (modal) {
    installModalFocusState(modal, options);
    modal.classList.add(VISIBLE_MODAL_CLASS);
    modal.setAttribute('aria-hidden', 'false');
    modal.setAttribute('aria-modal', 'true');
    ensureModalRole(modal);
    ensureModalFocusable(modal);
    scheduleModalFocus(modal, options);
  }
  return modal;
}

export function hideModal(id: string): HTMLElement | null {
  const modal = document.getElementById(id);
  if (modal) {
    modal.classList.remove(VISIBLE_MODAL_CLASS);
    modal.setAttribute('aria-hidden', 'true');
    modal.removeAttribute('aria-modal');
    removeModalFocusState(modal, true);
  }
  return modal;
}

function scheduleModalFocus(modal: HTMLElement, options: ShowModalOptions): void {
  if (!shouldFocusModal(options)) {
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
  if (!modal.classList.contains(VISIBLE_MODAL_CLASS)) {
    return;
  }

  const target = findInitialFocusTarget(modal, options);
  if (!target) {
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
    }
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

function installModalFocusState(modal: HTMLElement, options: ShowModalOptions): void {
  removeModalFocusState(modal, false);

  const state: ModalFocusState = {
    modal,
    previousFocus: document.activeElement,
    restoreFocus: options.restoreFocus ?? true,
    trapFocus: options.trapFocus ?? true,
    handleKeydown: event => trapModalFocus(state, event)
  };

  activeModalStack.push(state);
  modalFocusStates.set(modal, state);
  document.addEventListener('keydown', state.handleKeydown, true);
}

function removeModalFocusState(modal: HTMLElement, restoreFocus: boolean): void {
  const state = modalFocusStates.get(modal);
  if (!state) {
    return;
  }

  document.removeEventListener('keydown', state.handleKeydown, true);
  modalFocusStates.delete(modal);

  const index = activeModalStack.indexOf(state);
  if (index >= 0) {
    activeModalStack.splice(index, 1);
  }

  if (restoreFocus && state.restoreFocus) {
    restorePreviousFocus(state.previousFocus);
  }
}

function trapModalFocus(state: ModalFocusState, event: KeyboardEvent): void {
  if (
    event.key !== 'Tab' ||
    !state.trapFocus ||
    getTopModalState() !== state ||
    !document.contains(state.modal) ||
    !state.modal.classList.contains(VISIBLE_MODAL_CLASS)
  ) {
    return;
  }

  const focusableElements = getFocusableElements(state.modal);
  if (focusableElements.length === 0) {
    event.preventDefault();
    state.modal.focus();
    return;
  }

  const firstElement = focusableElements[0];
  const lastElement = focusableElements[focusableElements.length - 1];
  const activeElement = document.activeElement;

  if (!state.modal.contains(activeElement)) {
    event.preventDefault();
    (event.shiftKey ? lastElement : firstElement).focus();
    return;
  }

  if (event.shiftKey && activeElement === firstElement) {
    event.preventDefault();
    lastElement.focus();
    return;
  }

  if (!event.shiftKey && activeElement === lastElement) {
    event.preventDefault();
    firstElement.focus();
  }
}

function getTopModalState(): ModalFocusState | null {
  return activeModalStack[activeModalStack.length - 1] ?? null;
}

function restorePreviousFocus(previousFocus: Element | null): void {
  if (!(previousFocus instanceof HTMLElement) || !document.contains(previousFocus)) {
    return;
  }

  if (previousFocus.closest('.modal[aria-hidden="true"]')) {
    return;
  }

  try {
    previousFocus.focus();
  } catch {
    // Some environments expose focusable elements that cannot receive focus.
  }
}

function findInitialFocusTarget(modal: HTMLElement, options: ShowModalOptions): HTMLElement | null {
  if (options.focusSelector) {
    const selectedTarget = modal.querySelector<HTMLElement>(options.focusSelector);
    if (selectedTarget && isFocusableElement(selectedTarget)) {
      return selectedTarget;
    }
  }

  if (options.focusFirst === false) {
    return null;
  }

  return getFocusableElements(modal)[0] ?? modal;
}

function getFocusableElements(modal: HTMLElement): HTMLElement[] {
  return Array.from(modal.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR))
    .filter(isFocusableElement);
}

function isFocusableElement(element: HTMLElement): boolean {
  if (isDisabledControl(element) || element.hidden || element.getAttribute('aria-hidden') === 'true') {
    return false;
  }

  if (element.closest('[hidden], [aria-hidden="true"]')) {
    return false;
  }

  const tabindex = element.getAttribute('tabindex');
  return tabindex === null || Number.parseInt(tabindex, 10) >= 0;
}

function shouldFocusModal(options: ShowModalOptions): boolean {
  return options.focusSelector !== undefined || options.focusFirst !== false;
}

function ensureModalRole(modal: HTMLElement): void {
  if (!modal.hasAttribute('role')) {
    modal.setAttribute('role', 'dialog');
  }
}

function ensureModalFocusable(modal: HTMLElement): void {
  if (!modal.hasAttribute('tabindex')) {
    modal.setAttribute('tabindex', '-1');
  }
}
