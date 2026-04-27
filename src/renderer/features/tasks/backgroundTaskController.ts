import type { Task } from '../../../shared/types';
import {
  findTaskById,
  flattenTasks
} from './taskTree';

type TimerHandle = ReturnType<typeof setTimeout>;
type LogSink = Pick<Console, 'log'>;

export interface BackgroundTaskControllerOptions {
  getTasks: () => Task[];
  onChanged: () => void;
  onActivated?: (count: number) => void;
  logger?: LogSink;
}

export interface BackgroundTaskController {
  activateTriggeredTasks: (task: Task) => void;
  deactivateTriggeredTasks: (task: Task) => void;
  restartExpirationTimers: () => void;
  clearExpirationTimer: (taskId: number) => void;
  clearAllExpirationTimers: () => void;
}

export function createBackgroundTaskController(
  options: BackgroundTaskControllerOptions
): BackgroundTaskController {
  const expirationTimers: Record<number, TimerHandle> = {};
  const logger = options.logger || console;

  function activateTriggeredTasks(task: Task): void {
    if (task.triggers.length === 0) {
      return;
    }

    let activatedCount = 0;
    task.triggers.forEach(triggeredId => {
      const backgroundTask = findTaskById(triggeredId, options.getTasks());
      if (backgroundTask?.mode === 'background' && !backgroundTask.activated) {
        backgroundTask.activated = true;
        backgroundTask.activatedAt = new Date().toISOString();
        activatedCount++;
        startExpirationTimer(backgroundTask);
      }
    });

    if (activatedCount > 0) {
      options.onActivated?.(activatedCount);
    }
  }

  function deactivateTriggeredTasks(task: Task): void {
    if (task.triggers.length === 0) {
      return;
    }

    task.triggers.forEach(triggeredId => {
      const backgroundTask = findTaskById(triggeredId, options.getTasks());
      if (backgroundTask?.mode === 'background' && backgroundTask.activated && !backgroundTask.completed) {
        backgroundTask.activated = false;
        backgroundTask.activatedAt = null;
        clearExpirationTimer(backgroundTask.id);
      }
    });
  }

  function restartExpirationTimers(): void {
    flattenTasks(options.getTasks())
      .filter(task => task.mode === 'background' && task.activated && !task.completed)
      .forEach(task => startExpirationTimer(task));
  }

  function startExpirationTimer(backgroundTask: Task): void {
    const expiresAfterMinutes = backgroundTask.backgroundOptions?.expiresAfterMinutes;
    if (!expiresAfterMinutes) {
      return;
    }

    clearExpirationTimer(backgroundTask.id);

    const delayMs = calculateExpirationDelayMs(backgroundTask, expiresAfterMinutes);
    if (delayMs <= 0) {
      backgroundTask.activated = false;
      options.onChanged();
      return;
    }

    expirationTimers[backgroundTask.id] = setTimeout(() => {
      if (backgroundTask.activated && !backgroundTask.completed) {
        backgroundTask.activated = false;
        options.onChanged();
        logger.log(`Background task "${backgroundTask.text}" expired`);
      }
      clearExpirationTimer(backgroundTask.id);
    }, delayMs);
  }

  function clearExpirationTimer(taskId: number): void {
    const timer = expirationTimers[taskId];
    if (timer) {
      clearTimeout(timer);
      delete expirationTimers[taskId];
    }
  }

  function clearAllExpirationTimers(): void {
    Object.keys(expirationTimers).forEach(taskId => {
      clearExpirationTimer(Number(taskId));
    });
  }

  return {
    activateTriggeredTasks,
    deactivateTriggeredTasks,
    restartExpirationTimers,
    clearExpirationTimer,
    clearAllExpirationTimers
  };
}

function calculateExpirationDelayMs(backgroundTask: Task, expiresAfterMinutes: number): number {
  const durationMs = expiresAfterMinutes * 60 * 1000;
  if (!backgroundTask.activatedAt) {
    return durationMs;
  }

  const activatedAtMs = new Date(backgroundTask.activatedAt).getTime();
  if (!Number.isFinite(activatedAtMs)) {
    return durationMs;
  }

  return durationMs - (Date.now() - activatedAtMs);
}
