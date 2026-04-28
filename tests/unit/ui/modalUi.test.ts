import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import {
  hideModal,
  showModal
} from '../../../src/renderer/ui/modalUi';

describe('modal UI helpers', () => {
  beforeEach(() => {
    vi.spyOn(window, 'focus').mockImplementation(() => undefined);
  });

  afterEach(() => {
    hideModal('settingsModal');
    vi.restoreAllMocks();
    vi.useRealTimers();
    document.body.replaceChildren();
  });

  test('toggles modal visibility class and aria state', () => {
    document.body.innerHTML = '<div id="settingsModal" class="modal"></div>';

    expect(showModal('settingsModal')).toBe(getModal());
    expect(getModal().classList.contains('is-visible')).toBe(true);
    expect(getModal().getAttribute('aria-hidden')).toBe('false');
    expect(getModal().getAttribute('aria-modal')).toBe('true');
    expect(getModal().getAttribute('role')).toBe('dialog');

    expect(hideModal('settingsModal')).toBe(getModal());
    expect(getModal().classList.contains('is-visible')).toBe(false);
    expect(getModal().getAttribute('aria-hidden')).toBe('true');
    expect(getModal().hasAttribute('aria-modal')).toBe(false);
  });

  test('returns null for missing modals', () => {
    expect(showModal('missingModal')).toBeNull();
    expect(hideModal('missingModal')).toBeNull();
  });

  test('focuses the requested control after the modal becomes visible', () => {
    vi.useFakeTimers();
    const focusWindow = vi.fn();
    document.body.innerHTML = `
      <div id="settingsModal" class="modal">
        <input id="templateNameInput" value="Boss setup">
      </div>
    `;

    showModal('settingsModal', {
      focusSelector: '#templateNameInput',
      focusWindow,
      selectText: true
    });

    vi.advanceTimersByTime(50);

    const input = getInput('templateNameInput');
    expect(document.activeElement).toBe(input);
    expect(input.selectionStart).toBe(0);
    expect(input.selectionEnd).toBe('Boss setup'.length);
    expect(focusWindow).toHaveBeenCalledOnce();
  });

  test('focuses the first focusable control when no selector is provided', () => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <div id="settingsModal" class="modal">
        <button id="firstButton">First</button>
        <button id="secondButton">Second</button>
      </div>
    `;

    showModal('settingsModal');
    vi.advanceTimersByTime(50);

    expect(document.activeElement).toBe(getButton('firstButton'));
  });

  test('keeps tab focus inside the topmost modal', () => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <div id="settingsModal" class="modal">
        <button id="firstButton">First</button>
        <button id="secondButton">Second</button>
      </div>
    `;

    showModal('settingsModal');
    vi.advanceTimersByTime(50);

    getButton('secondButton').focus();
    dispatchTab();
    expect(document.activeElement).toBe(getButton('firstButton'));

    dispatchTab({ shiftKey: true });
    expect(document.activeElement).toBe(getButton('secondButton'));
  });

  test('restores focus to the opener when the modal closes', () => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <button id="beforeModal">Before</button>
      <div id="settingsModal" class="modal">
        <button id="firstButton">First</button>
      </div>
    `;

    const opener = getButton('beforeModal');
    opener.focus();
    showModal('settingsModal');
    vi.advanceTimersByTime(50);

    expect(document.activeElement).toBe(getButton('firstButton'));
    hideModal('settingsModal');

    expect(document.activeElement).toBe(opener);
  });

  test('does not steal focus after a modal has already closed', () => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <button id="beforeModal">Before</button>
      <div id="settingsModal" class="modal">
        <input id="templateNameInput">
      </div>
    `;

    const button = getButton('beforeModal');
    button.focus();
    showModal('settingsModal', { focusSelector: '#templateNameInput' });
    hideModal('settingsModal');

    vi.runOnlyPendingTimers();

    expect(document.activeElement).toBe(button);
  });
});

function getModal(): HTMLElement {
  const element = document.getElementById('settingsModal');
  if (!element) {
    throw new Error('Missing modal fixture');
  }
  return element;
}

function getInput(id: string): HTMLInputElement {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`Missing input ${id}`);
  }
  return element;
}

function getButton(id: string): HTMLButtonElement {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLButtonElement)) {
    throw new Error(`Missing button ${id}`);
  }
  return element;
}

function dispatchTab(options: { shiftKey?: boolean } = {}): void {
  document.dispatchEvent(new KeyboardEvent('keydown', {
    bubbles: true,
    cancelable: true,
    key: 'Tab',
    shiftKey: options.shiftKey
  }));
}
