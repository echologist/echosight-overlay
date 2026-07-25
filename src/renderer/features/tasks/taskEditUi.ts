import type { Task } from '../../../shared/types';
import { hideModal, showModal } from '../../ui/modalUi';
import {
  focusOverlayNow,
  type FocusOverlayApi
} from '../../ui/windowFocus';
import {
  readCheckboxChecked,
  readTrimmedInputValue
} from './backgroundTaskUi';

export interface TaskEditFormValue {
  text: string;
  highPriority: boolean;
}

export function showTaskEditModal(task: Task, api?: FocusOverlayApi): boolean {
  const modal = document.getElementById('taskEditModal');
  const input = document.getElementById('taskEditInput');
  const backgroundOptions = document.getElementById('taskEditBackgroundOptions');
  const highPriority = document.getElementById('taskEditHighPriority');
  if (
    !modal ||
    !(input instanceof HTMLInputElement) ||
    !backgroundOptions ||
    !(highPriority instanceof HTMLInputElement)
  ) {
    return false;
  }

  input.value = task.text;
  backgroundOptions.hidden = task.mode !== 'background';
  highPriority.checked = task.backgroundOptions?.priority === 'high';
  showModal('taskEditModal', {
    focusSelector: '#taskEditInput',
    focusWindow: () => focusOverlayNow(api),
    selectText: true
  });
  return true;
}

export function closeTaskEditModal(): void {
  hideModal('taskEditModal');
}

export function readTaskEditForm(): TaskEditFormValue {
  return {
    text: readTrimmedInputValue(document.getElementById('taskEditInput')),
    highPriority: readCheckboxChecked(document.getElementById('taskEditHighPriority'))
  };
}
