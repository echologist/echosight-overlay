import type { FocusOverlayApi } from '../../ui/windowFocus';
import { hideModal, showModal } from '../../ui/modalUi';
import {
  focusOverlayNow,
  focusOverlaySoon
} from '../../ui/windowFocus';

export function showSubTaskModal(api?: FocusOverlayApi): void {
  showModal('addSubTaskModal', {
    focusDelayMs: 150,
    focusSelector: '#subTaskInput',
    focusWindow: () => {
      focusOverlayNow(api);
    },
    selectText: true
  });
}

export function closeSubTaskModal(api?: FocusOverlayApi): void {
  hideModal('addSubTaskModal');
  setSubTaskInputValue('');

  if (api) {
    focusOverlaySoon(api, 50);
  } else {
    setTimeout(() => window.focus?.(), 50);
  }
}

export function readSubTaskInput(): string {
  return getSubTaskInput()?.value.trim() || '';
}

function setSubTaskInputValue(value: string): void {
  const input = getSubTaskInput();
  if (input) {
    input.value = value;
  }
}

function getSubTaskInput(): HTMLInputElement | null {
  const element = document.getElementById('subTaskInput');
  return element instanceof HTMLInputElement ? element : null;
}
