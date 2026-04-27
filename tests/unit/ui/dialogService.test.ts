import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { createDialogService } from '../../../src/renderer/ui/dialogService';

describe('dialog service', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(window, 'focus').mockImplementation(() => undefined);
    document.body.innerHTML = `
      <div class="modal app-dialog-modal" id="appDialogModal" aria-hidden="true">
        <div class="modal-content modal-content-dialog app-dialog-tone-info">
          <h3 class="modal-title" id="appDialogTitle"></h3>
          <p class="app-dialog-message" id="appDialogMessage"></p>
          <div class="modal-buttons">
            <button class="modal-btn secondary" id="appDialogCancelButton">Cancel</button>
            <button class="modal-btn primary" id="appDialogConfirmButton">OK</button>
          </div>
        </div>
      </div>
    `;
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    vi.restoreAllMocks();
    document.body.replaceChildren();
  });

  test('shows a themed alert and resolves on OK', async () => {
    const dialogs = createDialogService();
    const result = dialogs.alert('Saved successfully', {
      title: 'Saved',
      tone: 'success'
    });

    await allowDialogToOpen();

    expect(getModal().classList.contains('is-visible')).toBe(true);
    expect(getTitle().textContent).toBe('Saved');
    expect(getMessage().textContent).toBe('Saved successfully');
    expect(getCancelButton().hidden).toBe(true);
    expect(getContent().classList.contains('app-dialog-tone-success')).toBe(true);

    getConfirmButton().click();
    await result;

    expect(getModal().classList.contains('is-visible')).toBe(false);
  });

  test('resolves confirm actions from button clicks', async () => {
    const dialogs = createDialogService();
    const cancelled = dialogs.confirm('Delete task?', {
      title: 'Delete Task',
      tone: 'danger'
    });

    await allowDialogToOpen();

    expect(getTitle().textContent).toBe('Delete Task');
    expect(getCancelButton().hidden).toBe(false);
    expect(getContent().classList.contains('app-dialog-tone-danger')).toBe(true);

    getCancelButton().click();
    await expect(cancelled).resolves.toBe(false);

    const confirmed = dialogs.confirm('Delete task?');
    await allowDialogToOpen();
    getConfirmButton().click();

    await expect(confirmed).resolves.toBe(true);
  });

  test('queues dialogs so only one prompt is active at a time', async () => {
    const dialogs = createDialogService();
    const first = dialogs.confirm('First');
    const second = dialogs.confirm('Second');

    await allowDialogToOpen();
    expect(getMessage().textContent).toBe('First');

    getConfirmButton().click();
    await expect(first).resolves.toBe(true);
    await allowDialogToOpen();

    expect(getMessage().textContent).toBe('Second');
    getCancelButton().click();
    await expect(second).resolves.toBe(false);
  });
});

async function allowDialogToOpen(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}

function getModal(): HTMLElement {
  return getElement('appDialogModal');
}

function getContent(): HTMLElement {
  const content = getModal().querySelector<HTMLElement>('.modal-content-dialog');
  if (!content) {
    throw new Error('Missing dialog content');
  }
  return content;
}

function getTitle(): HTMLElement {
  return getElement('appDialogTitle');
}

function getMessage(): HTMLElement {
  return getElement('appDialogMessage');
}

function getCancelButton(): HTMLButtonElement {
  return getButton('appDialogCancelButton');
}

function getConfirmButton(): HTMLButtonElement {
  return getButton('appDialogConfirmButton');
}

function getButton(id: string): HTMLButtonElement {
  const element = getElement(id);
  if (!(element instanceof HTMLButtonElement)) {
    throw new Error(`Missing button ${id}`);
  }
  return element;
}

function getElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing element ${id}`);
  }
  return element;
}
