import { describe, expect, test, vi } from 'vitest';
import { ensureSingleInstance } from '../../../src/main/lifecycle/singleInstance';

describe('single-instance lifecycle', () => {
  test('quits and reports denial when lock is unavailable', () => {
    const harness = createAppHarness(false);
    const onSecondInstance = vi.fn();

    expect(ensureSingleInstance(harness.app, onSecondInstance)).toBe(false);
    expect(harness.app.quit).toHaveBeenCalledOnce();
    expect(harness.app.on).not.toHaveBeenCalled();
    expect(onSecondInstance).not.toHaveBeenCalled();
  });

  test('registers and invokes the supplied second-instance callback', () => {
    const harness = createAppHarness(true);
    const onSecondInstance = vi.fn();

    expect(ensureSingleInstance(harness.app, onSecondInstance)).toBe(true);
    expect(harness.app.quit).not.toHaveBeenCalled();
    expect(harness.app.on).toHaveBeenCalledWith('second-instance', onSecondInstance);

    harness.emitSecondInstance();

    expect(onSecondInstance).toHaveBeenCalledOnce();
  });
});

function createAppHarness(lockGranted: boolean) {
  let secondInstanceListener: (() => void) | undefined;
  const app = {
    requestSingleInstanceLock: vi.fn(() => lockGranted),
    quit: vi.fn(),
    on: vi.fn((event: string, listener: () => void) => {
      if (event === 'second-instance') {
        secondInstanceListener = listener;
      }
    })
  };

  return {
    app,
    emitSecondInstance: () => {
      if (!secondInstanceListener) {
        throw new Error('second-instance listener was not registered');
      }
      secondInstanceListener();
    }
  };
}
