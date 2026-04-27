export const layoutSelectorMap: Record<string, string[]> = {
  header: ['.header'],
  templateSection: ['.template-section'],
  tasksSection: ['.tasks-section'],
  taskList: ['.tasks-list'],
  taskItem: ['.task-item'],
  progressBar: ['.progress-bar'],
  buttons: ['button', '.template-btn', '.add-btn', '.header-btn'],
  inputs: ['input', 'textarea', 'select']
};

export const animationSelectorMap: Record<string, string[]> = {
  container: ['.overlay-container'],
  header: ['.header'],
  tasks: ['.task-item'],
  buttons: ['button', '.template-btn', '.add-btn'],
  progress: ['.progress-bar', '.progress-fill'],
  modal: ['.modal', '.settings-modal'],
  inputs: ['input', 'textarea', 'select']
};

export const componentSelectorMap: Record<string, string[]> = {
  button: [
    'button',
    '.template-btn',
    '.add-btn',
    '.header-btn',
    '.modal-btn'
  ],
  input: [
    'input[type="text"]',
    'input[type="password"]',
    'textarea',
    '.task-input',
    '.modal-input'
  ],
  select: [
    'select',
    '.template-select'
  ],
  progressBar: [
    '.progress-bar'
  ],
  progressFill: [
    '.progress-fill'
  ],
  taskItem: [
    '.task-item'
  ],
  checkbox: [
    'input[type="checkbox"]',
    '.task-checkbox'
  ]
};

export const animationPropertyMap: Record<string, string> = {
  name: 'animation-name',
  duration: 'animation-duration',
  timing: 'animation-timing-function',
  delay: 'animation-delay',
  iteration: 'animation-iteration-count',
  direction: 'animation-direction',
  fillMode: 'animation-fill-mode',
  playState: 'animation-play-state',
  transitionTiming: 'transition-timing-function'
};
