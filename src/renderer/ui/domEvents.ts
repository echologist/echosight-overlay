type MaybePromise<T> = T | Promise<T>;

export function bindClick(id: string, handler: () => MaybePromise<void>): void {
  const element = document.getElementById(id);
  if (element) {
    element.addEventListener('click', () => runSafely(handler));
  }
}

export function bindEnterKey(
  id: string,
  handler: () => MaybePromise<void>,
  options: { primaryModifier?: boolean } = {}
): void {
  const element = document.getElementById(id);
  if (!element) {
    return;
  }

  element.addEventListener('keydown', event => {
    const hasPrimaryModifier = event.ctrlKey || event.metaKey;
    if (event.key === 'Enter' && (!options.primaryModifier || hasPrimaryModifier)) {
      runSafely(handler);
    }
  });
}

export function bindValueInput(id: string, handler: (value: string) => MaybePromise<void>): void {
  const element = document.getElementById(id);
  if (!isValueControl(element)) {
    return;
  }

  element.addEventListener('input', event => {
    const control = event.currentTarget;
    if (isValueControl(control)) {
      const { value } = control;
      runSafely(() => handler(value));
    }
  });
}

export function bindValueChange(id: string, handler: (value: string) => MaybePromise<void>): void {
  const element = document.getElementById(id);
  if (!isValueControl(element)) {
    return;
  }

  element.addEventListener('change', event => {
    const control = event.currentTarget;
    if (isValueControl(control)) {
      const { value } = control;
      runSafely(() => handler(value));
    }
  });
}

export function bindEvent(id: string, eventName: string, handler: (event: Event) => MaybePromise<void>): void {
  const element = document.getElementById(id);
  if (element) {
    element.addEventListener(eventName, event => runSafely(() => handler(event)));
  }
}

export function clickElement(id: string): void {
  document.getElementById(id)?.click();
}

export function isOutsideModalContent(event: Event): boolean {
  return event.target instanceof Element && !event.target.closest('.modal-content');
}

function isValueControl(element: EventTarget | null): element is HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement {
  return element instanceof HTMLInputElement ||
    element instanceof HTMLTextAreaElement ||
    element instanceof HTMLSelectElement;
}

function runSafely(handler: () => MaybePromise<void>): void {
  try {
    const result = handler();
    if (isPromiseLike(result)) {
      void result.catch(error => console.error('Unhandled DOM event handler error:', error));
    }
  } catch (error) {
    console.error('Unhandled DOM event handler error:', error);
  }
}

function isPromiseLike(value: unknown): value is Promise<void> {
  return !!value && typeof (value as Promise<void>).catch === 'function';
}
