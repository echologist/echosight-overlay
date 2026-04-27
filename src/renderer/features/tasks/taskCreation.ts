import type {
  BackgroundOptions,
  Task
} from '../../../shared/types';
import { isRecord } from './taskGuards';

export function createTaskId(): number {
  return Date.now() * 1000 + Math.floor(Math.random() * 1000);
}

export function sanitizeBackgroundOptions(options: unknown): BackgroundOptions | null {
  if (!isRecord(options)) {
    return null;
  }

  const expiresAfterMinutes = Number(options.expiresAfterMinutes);

  return {
    expiresAfterMinutes: Number.isFinite(expiresAfterMinutes) && expiresAfterMinutes > 0
      ? Math.floor(expiresAfterMinutes)
      : null,
    priority: options.priority === 'high' ? 'high' : 'normal'
  };
}

export function createTask(text: string, parentTask: Task | null = null): Task {
  const mode = parentTask?.mode || 'main';

  return {
    id: createTaskId(),
    text,
    completed: false,
    createdAt: new Date().toISOString(),
    children: [],
    mode,
    triggers: [],
    activated: mode === 'background' ? !!parentTask?.activated : true,
    activatedAt: null,
    backgroundOptions: null
  };
}

export function createBackgroundTask(text: string, options: unknown = {}): Task {
  return {
    id: createTaskId(),
    text,
    completed: false,
    createdAt: new Date().toISOString(),
    children: [],
    mode: 'background',
    triggers: [],
    activated: false,
    activatedAt: null,
    backgroundOptions: sanitizeBackgroundOptions(options) || {
      expiresAfterMinutes: null,
      priority: 'normal'
    }
  };
}
