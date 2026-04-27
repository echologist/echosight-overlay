import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import type { Task } from '../../../src/shared/types';
import { showTriggerConfigModal } from '../../../src/renderer/features/tasks/backgroundTaskUi';

describe('background task UI', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.spyOn(window, 'focus').mockImplementation(() => undefined);
    document.body.innerHTML = `
      <div id="configureTriggersModal" class="modal">
        <div id="triggerTaskList"></div>
        <input id="newBgTaskInput" value="Prepare arena">
        <input id="bgTaskHighPriority" type="checkbox">
      </div>
    `;
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
    document.body.replaceChildren();
  });

  test('focuses the new background task input when opening configure triggers', () => {
    const focusWindow = vi.fn();

    expect(showTriggerConfigModal(createTask(1, 'Main task'), [], { focusWindow })).toBe(true);

    vi.advanceTimersByTime(50);

    const input = getInput('newBgTaskInput');
    expect(document.activeElement).toBe(input);
    expect(input.selectionStart).toBe(0);
    expect(input.selectionEnd).toBe(0);
    expect(focusWindow).toHaveBeenCalledOnce();
  });
});

function createTask(id: number, text: string): Task {
  return {
    id,
    text,
    completed: false,
    createdAt: '2026-04-27T00:00:00.000Z',
    children: [],
    mode: 'main',
    triggers: [],
    activated: true,
    activatedAt: null,
    backgroundOptions: null
  };
}

function getInput(id: string): HTMLInputElement {
  const element = document.getElementById(id);
  if (!(element instanceof HTMLInputElement)) {
    throw new Error(`Missing input ${id}`);
  }
  return element;
}
