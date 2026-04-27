import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import {
  renderTaskProgress,
  showCompletedTaskFeedback
} from '../../../src/renderer/features/tasks/taskProgressUi';

describe('task progress UI', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <div id="progressFill"></div>
      <div id="progressText"></div>
    `;
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    document.body.replaceChildren();
  });

  test('renders progress and temporary completion feedback', () => {
    renderTaskProgress({ completed: 1, total: 3, percentage: 33 });
    expect(getProgressFill().style.width).toBe('33%');
    expect(getProgressText().textContent).toBe('1 / 3 tasks completed');

    showCompletedTaskFeedback('Sharpen blade');
    expect(getProgressText().textContent).toBe('✓ Completed: Sharpen blade');
    expect(getProgressText().classList.contains('completed-feedback')).toBe(true);

    vi.advanceTimersByTime(2000);
    expect(getProgressText().textContent).toBe('1 / 3 tasks completed');
    expect(getProgressText().classList.contains('completed-feedback')).toBe(false);
  });

  test('progress renders cancel stale feedback timers', () => {
    renderTaskProgress({ completed: 1, total: 3, percentage: 33 });
    showCompletedTaskFeedback('First');

    renderTaskProgress({ completed: 2, total: 3, percentage: 67 });
    vi.advanceTimersByTime(2000);

    expect(getProgressFill().style.width).toBe('67%');
    expect(getProgressText().textContent).toBe('2 / 3 tasks completed');
    expect(getProgressText().classList.contains('completed-feedback')).toBe(false);
  });
});

function getProgressFill(): HTMLElement {
  return getElement('progressFill');
}

function getProgressText(): HTMLElement {
  return getElement('progressText');
}

function getElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing element ${id}`);
  }
  return element;
}
