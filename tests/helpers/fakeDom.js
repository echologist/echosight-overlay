class FakeClassList {
  constructor(element) {
    this.element = element;
  }

  values() {
    return new Set(String(this.element.className || '').split(/\s+/).filter(Boolean));
  }

  write(values) {
    this.element.className = Array.from(values).join(' ');
  }

  add(...classNames) {
    const values = this.values();
    classNames.forEach(name => values.add(name));
    this.write(values);
  }

  remove(...classNames) {
    const values = this.values();
    classNames.forEach(name => values.delete(name));
    this.write(values);
  }

  contains(className) {
    return this.values().has(className);
  }
}

function matchesSelector(element, selector) {
  if (!element || !selector) return false;

  if (selector.startsWith('.')) {
    return element.classList.contains(selector.slice(1));
  }

  if (selector.startsWith('#')) {
    return element.id === selector.slice(1);
  }

  const attrMatch = selector.match(/^([a-z]+)?\[([^=]+)="?([^"]+)"?\]$/i);
  if (attrMatch) {
    const [, tagName, attrName, attrValue] = attrMatch;
    const tagMatches = !tagName || element.tagName.toLowerCase() === tagName.toLowerCase();
    return tagMatches && String(element[attrName] || element.getAttribute(attrName) || '') === attrValue;
  }

  return element.tagName.toLowerCase() === selector.toLowerCase();
}

class FakeElement {
  constructor(tagName = 'div') {
    this.tagName = tagName.toUpperCase();
    this.children = [];
    this.parentNode = null;
    this.style = {};
    this.dataset = {};
    this.attributes = {};
    this.listeners = {};
    this.className = '';
    this.classList = new FakeClassList(this);
    this.textContent = '';
    this.value = '';
    this.checked = false;
    this.disabled = false;
    this.draggable = false;
    this.hasDropHandler = false;
    this._innerHTML = '';
    this._rect = { top: 0, left: 0, width: 240, height: 32 };
  }

  set id(value) {
    this._id = value;
  }

  get id() {
    return this._id;
  }

  set innerHTML(value) {
    this._innerHTML = String(value);
    if (value === '') {
      this.children = [];
    }
  }

  get innerHTML() {
    return this._innerHTML;
  }

  setAttribute(name, value) {
    this.attributes[name] = String(value);
    if (name === 'class') this.className = String(value);
    if (name === 'id') this.id = String(value);
  }

  getAttribute(name) {
    return this.attributes[name];
  }

  appendChild(child) {
    child.parentNode = this;
    this.children.push(child);
    return child;
  }

  removeChild(child) {
    const index = this.children.indexOf(child);
    if (index >= 0) {
      this.children.splice(index, 1);
      child.parentNode = null;
    }
    return child;
  }

  insertBefore(child, referenceChild) {
    child.parentNode = this;
    const index = this.children.indexOf(referenceChild);
    if (index === -1) {
      this.children.push(child);
    } else {
      this.children.splice(index, 0, child);
    }
    return child;
  }

  remove() {
    if (this.parentNode) {
      this.parentNode.removeChild(this);
    }
  }

  addEventListener(type, listener) {
    this.listeners[type] = this.listeners[type] || [];
    this.listeners[type].push(listener);
  }

  dispatchEvent(event) {
    const listeners = this.listeners[event.type] || [];
    listeners.forEach(listener => listener(event));
  }

  focus() {
    this.focused = true;
  }

  select() {
    this.selected = true;
  }

  closest(selector) {
    let current = this;
    while (current) {
      if (matchesSelector(current, selector)) return current;
      current = current.parentNode;
    }
    return null;
  }

  querySelectorAll(selector) {
    const matches = [];
    const visit = element => {
      if (matchesSelector(element, selector)) {
        matches.push(element);
      }
      element.children.forEach(visit);
    };

    this.children.forEach(visit);
    return matches;
  }

  querySelector(selector) {
    return this.querySelectorAll(selector)[0] || null;
  }

  getBoundingClientRect() {
    return this._rect;
  }
}

class FakeDocument {
  constructor() {
    this.elements = new Map();
    this.listeners = {};
    this.body = this.createElement('body');
    this.head = this.createElement('head');
  }

  createElement(tagName) {
    return new FakeElement(tagName);
  }

  registerElement(id, tagName = 'div') {
    const element = this.createElement(tagName);
    element.id = id;
    this.elements.set(id, element);
    this.body.appendChild(element);
    return element;
  }

  getElementById(id) {
    return this.elements.get(id) || null;
  }

  querySelector(selector) {
    return this.body.querySelector(selector) || this.head.querySelector(selector);
  }

  querySelectorAll(selector) {
    return [
      ...this.body.querySelectorAll(selector),
      ...this.head.querySelectorAll(selector)
    ];
  }

  addEventListener(type, listener) {
    this.listeners[type] = this.listeners[type] || [];
    this.listeners[type].push(listener);
  }

  dispatchEvent(event) {
    const listeners = this.listeners[event.type] || [];
    listeners.forEach(listener => listener(event));
  }
}

function createTestDocument() {
  const document = new FakeDocument();

  [
    ['taskInput', 'input'],
    ['taskList', 'ul'],
    ['backgroundTaskList', 'ul'],
    ['backgroundTasksSection', 'section'],
    ['backgroundCount', 'span'],
    ['progressFill', 'div'],
    ['progressText', 'span'],
    ['subTaskInput', 'input'],
    ['addSubTaskModal', 'div'],
    ['configureTriggersModal', 'div'],
    ['triggerTaskList', 'div'],
    ['newBgTaskInput', 'input'],
    ['bgTaskHighPriority', 'input'],
    ['settingsModal', 'div'],
    ['themeSelect', 'select'],
    ['templateSelect', 'select'],
    ['templateNameInput', 'input'],
    ['importTemplateInput', 'textarea'],
    ['saveTemplateModal', 'div'],
    ['importTemplateModal', 'div'],
    ['communityTemplatesModal', 'div'],
    ['communityTemplatesList', 'div'],
    ['transparencySlider', 'input'],
    ['transparencyValue', 'span'],
    ['toggleVisibilityHotkey', 'input'],
    ['toggleInteractiveHotkey', 'input'],
    ['completeNextTaskHotkey', 'input'],
    ['undoLastActionHotkey', 'input'],
    ['recordBtn1', 'button'],
    ['recordBtn2', 'button'],
    ['recordBtn3', 'button'],
    ['recordBtn4', 'button'],
    ['overlayContainer', 'div'],
    ['shortcut-hint', 'span'],
    ['interactiveToggle', 'button'],
    ['header', 'header']
  ].forEach(([id, tagName]) => document.registerElement(id, tagName));

  document.getElementById('bgTaskHighPriority').checked = false;
  document.getElementById('transparencySlider').value = '60';
  document.getElementById('themeSelect').value = 'echosight';
  document.getElementById('toggleVisibilityHotkey').value = 'Ctrl+Shift+T';
  document.getElementById('toggleInteractiveHotkey').value = 'Ctrl+Shift+I';
  document.getElementById('completeNextTaskHotkey').value = 'Ctrl+Shift+N';
  document.getElementById('undoLastActionHotkey').value = 'Ctrl+Shift+Z';

  return document;
}

module.exports = {
  FakeDocument,
  FakeElement,
  createTestDocument
};
