import type { CommunityTemplate } from './communityTemplates';
import { hideModal, showModal } from '../../ui/modalUi';

export interface CommunityTemplateActions {
  onImport: (index: number) => void;
  onPreview: (index: number) => void;
}

export function renderCommunityTemplateList(
  container: HTMLElement,
  templates: readonly CommunityTemplate[],
  actions: CommunityTemplateActions
): void {
  container.replaceChildren();

  templates.forEach((template, index) => {
    container.appendChild(createCommunityTemplateCard(template, index, actions));
  });
}

export function showCommunityTemplatesModal(
  templates: readonly CommunityTemplate[],
  actions: CommunityTemplateActions
): boolean {
  const modal = document.getElementById('communityTemplatesModal');
  const list = document.getElementById('communityTemplatesList');
  if (!modal || !list) {
    return false;
  }

  renderCommunityTemplateList(list, templates, actions);
  showModal('communityTemplatesModal');
  return true;
}

export function closeCommunityTemplatesModal(): void {
  hideModal('communityTemplatesModal');
}

export function createCommunityTemplatePreview(template: CommunityTemplate): string {
  const taskList = template.tasks.map((task, index) => `${index + 1}. ${task}`).join('\n');
  return `${template.name} Tasks:\n\n${taskList}`;
}

function createCommunityTemplateCard(
  template: CommunityTemplate,
  index: number,
  actions: CommunityTemplateActions
): HTMLDivElement {
  const card = document.createElement('div');
  card.className = 'community-template';

  card.appendChild(createTitle(template.name));
  card.appendChild(createDescription(template.description));
  card.appendChild(createTaskCount(template.tasks.length));
  card.appendChild(createActions(index, actions));

  return card;
}

function createTitle(name: string): HTMLHeadingElement {
  const title = document.createElement('h4');
  title.textContent = name;
  return title;
}

function createDescription(descriptionText: string): HTMLParagraphElement {
  const description = document.createElement('p');
  description.className = 'description';
  description.textContent = descriptionText;
  return description;
}

function createTaskCount(count: number): HTMLParagraphElement {
  const taskCount = document.createElement('p');
  taskCount.className = 'task-count';
  taskCount.textContent = `${count} tasks`;
  return taskCount;
}

function createActions(index: number, actions: CommunityTemplateActions): HTMLDivElement {
  const buttons = document.createElement('div');
  buttons.className = 'community-template-actions';

  buttons.appendChild(createButton('Add to My Templates', 'primary', () => actions.onImport(index)));
  buttons.appendChild(createButton('Preview Tasks', 'secondary', () => actions.onPreview(index)));

  return buttons;
}

function createButton(label: string, variant: 'primary' | 'secondary', onClick: () => void): HTMLButtonElement {
  const button = document.createElement('button');
  button.className = `modal-btn ${variant} community-template-action`;
  button.textContent = label;
  button.addEventListener('click', onClick);
  return button;
}
