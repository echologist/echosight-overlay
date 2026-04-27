import { hideModal, showModal } from './modalUi';
import type {
  DialogOptions,
  DialogTone
} from './dialogTypes';

const DIALOG_MODAL_ID = 'appDialogModal';
const DIALOG_CONFIRM_BUTTON_ID = 'appDialogConfirmButton';
const TONE_CLASSES = [
  'app-dialog-tone-info',
  'app-dialog-tone-danger',
  'app-dialog-tone-success'
] as const;

type DialogKind = 'alert' | 'confirm';

export interface DialogService {
  alert: (message: string, options?: DialogOptions) => Promise<void>;
  confirm: (message: string, options?: DialogOptions) => Promise<boolean>;
}

interface DialogElements {
  modal: HTMLElement;
  content: HTMLElement;
  title: HTMLElement;
  message: HTMLElement;
  cancelButton: HTMLButtonElement;
  confirmButton: HTMLButtonElement;
}

interface DialogRequest {
  kind: DialogKind;
  message: string;
  options: DialogOptions;
}

export function createDialogService(): DialogService {
  let queue = Promise.resolve();

  function enqueue<T>(task: () => Promise<T>): Promise<T> {
    const result = queue.then(task, task);
    queue = result.then(
      () => undefined,
      () => undefined
    );
    return result;
  }

  return {
    alert: (message, options = {}) =>
      enqueue(() => showDialog({
        kind: 'alert',
        message,
        options
      })).then(() => undefined),
    confirm: (message, options = {}) =>
      enqueue(() => showDialog({
        kind: 'confirm',
        message,
        options
      }))
  };
}

async function showDialog(request: DialogRequest): Promise<boolean> {
  const elements = getDialogElements();
  if (!elements) {
    return request.kind === 'alert';
  }

  configureDialog(elements, request);
  return new Promise(resolve => {
    let settled = false;

    const settle = (value: boolean): void => {
      if (settled) {
        return;
      }

      settled = true;
      elements.cancelButton.removeEventListener('click', cancel);
      elements.confirmButton.removeEventListener('click', accept);
      document.removeEventListener('keydown', handleKeydown, true);
      hideModal(DIALOG_MODAL_ID);
      resolve(value);
    };

    const cancel = (): void => {
      settle(request.kind === 'alert');
    };

    const accept = (): void => {
      settle(true);
    };

    const handleKeydown = (event: KeyboardEvent): void => {
      if (!elements.modal.classList.contains('is-visible')) {
        return;
      }

      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        cancel();
        return;
      }

      if (event.key === 'Enter' && !(event.target instanceof HTMLTextAreaElement)) {
        event.preventDefault();
        event.stopPropagation();
        accept();
      }
    };

    elements.cancelButton.addEventListener('click', cancel);
    elements.confirmButton.addEventListener('click', accept);
    document.addEventListener('keydown', handleKeydown, true);

    showModal(DIALOG_MODAL_ID, {
      focusSelector: `#${DIALOG_CONFIRM_BUTTON_ID}`
    });
  });
}

function configureDialog(elements: DialogElements, request: DialogRequest): void {
  const isConfirm = request.kind === 'confirm';
  const tone = request.options.tone ?? 'info';

  elements.title.textContent = request.options.title ?? (isConfirm ? 'Confirm Action' : 'Echosight');
  elements.message.textContent = request.message;
  elements.cancelButton.textContent = request.options.cancelLabel ?? 'Cancel';
  elements.confirmButton.textContent = request.options.confirmLabel ?? (isConfirm ? 'OK' : 'OK');
  elements.cancelButton.hidden = !isConfirm;

  TONE_CLASSES.forEach(className => elements.content.classList.remove(className));
  elements.content.classList.add(`app-dialog-tone-${tone}`);
}

function getDialogElements(): DialogElements | null {
  const modal = document.getElementById(DIALOG_MODAL_ID);
  const content = modal?.querySelector<HTMLElement>('.modal-content-dialog') ?? null;
  const title = document.getElementById('appDialogTitle');
  const message = document.getElementById('appDialogMessage');
  const cancelButton = document.getElementById('appDialogCancelButton');
  const confirmButton = document.getElementById(DIALOG_CONFIRM_BUTTON_ID);

  if (
    !modal ||
    !content ||
    !title ||
    !message ||
    !(cancelButton instanceof HTMLButtonElement) ||
    !(confirmButton instanceof HTMLButtonElement)
  ) {
    return null;
  }

  return {
    modal,
    content,
    title,
    message,
    cancelButton,
    confirmButton
  };
}
