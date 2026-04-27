import type { TaskTemplate } from '../../../shared/types';
import { hideModal, showModal } from '../../ui/modalUi';
import {
  focusOverlayNow,
  type FocusOverlayApi
} from '../../ui/windowFocus';
import { countTemplateTasks } from './templateOperations';

const TEMPLATE_PLACEHOLDER = 'Select Template...';
const IMPORT_PLACEHOLDER = 'Paste template JSON or share code here...';

export function showSaveTemplateModal(api?: FocusOverlayApi): void {
  showModal('saveTemplateModal', {
    focusSelector: '#templateNameInput',
    focusWindow: () => focusOverlayNow(api),
    selectText: true
  });
}

export function closeSaveTemplateModal(): void {
  hideModal('saveTemplateModal');
  setTextControlValue('templateNameInput', '');
}

export function readTemplateName(): string {
  return readTextControlValue('templateNameInput');
}

export function showImportTemplateModal(api?: FocusOverlayApi): void {
  const input = getTextControl('importTemplateInput');
  if (input) {
    input.placeholder = IMPORT_PLACEHOLDER;
  }

  showModal('importTemplateModal', {
    focusSelector: '#importTemplateInput',
    focusWindow: () => focusOverlayNow(api)
  });
}

export function closeImportTemplateModal(): void {
  hideModal('importTemplateModal');
  setTextControlValue('importTemplateInput', '');
  setFileInputValue('importFileInput', '');
}

export function readImportTemplateInput(): string {
  return readTextControlValue('importTemplateInput');
}

export function writeImportTemplateInput(value: string): void {
  setTextControlValue('importTemplateInput', value);
}

export function readSelectedTemplateId(): string {
  return readSelectValue('templateSelect');
}

export function renderTemplateSelect(templates: TaskTemplate[]): void {
  const select = getSelectElement('templateSelect');
  if (!select) {
    return;
  }

  select.replaceChildren();
  const placeholder = document.createElement('option');
  placeholder.value = '';
  placeholder.textContent = TEMPLATE_PLACEHOLDER;
  select.appendChild(placeholder);

  templates.forEach(template => {
    const option = document.createElement('option');
    option.value = String(template.id);
    option.textContent = `${template.name} (${countTemplateTasks(template.tasks)} tasks)`;
    select.appendChild(option);
  });
}

export function downloadJsonFile(filename: string, data: unknown): void {
  const dataBlob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
  const url = URL.createObjectURL(dataBlob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  document.body.appendChild(link);
  link.click();
  link.remove();
  URL.revokeObjectURL(url);
}

export function readFirstSelectedFile(event: Event): File | null {
  const input = event.target;
  if (!(input instanceof HTMLInputElement)) {
    return null;
  }

  return input.files?.[0] || null;
}

export function readFileText(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();

    reader.onload = event => {
      resolve(typeof event.target?.result === 'string' ? event.target.result : '');
    };
    reader.onerror = () => {
      reject(reader.error || new Error('Failed to read file'));
    };

    reader.readAsText(file);
  });
}

function readTextControlValue(id: string): string {
  return getTextControl(id)?.value.trim() || '';
}

function setTextControlValue(id: string, value: string): void {
  const input = getTextControl(id);
  if (input) {
    input.value = value;
  }
}

function setFileInputValue(id: string, value: string): void {
  const input = document.getElementById(id);
  if (input instanceof HTMLInputElement) {
    input.value = value;
  }
}

function readSelectValue(id: string): string {
  return getSelectElement(id)?.value || '';
}

function getTextControl(id: string): HTMLInputElement | HTMLTextAreaElement | null {
  const element = document.getElementById(id);
  return element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement
    ? element
    : null;
}

function getSelectElement(id: string): HTMLSelectElement | null {
  const element = document.getElementById(id);
  return element instanceof HTMLSelectElement ? element : null;
}
