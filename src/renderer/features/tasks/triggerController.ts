import type {
  BackgroundPriority,
  Task
} from '../../../shared/types';
import {
  closeTriggerConfigModal,
  readNewBackgroundTaskInput,
  readSelectedTriggerIds,
  showTriggerConfigModal
} from './backgroundTaskUi';
import { flattenTasks } from './taskTree';
import type { FocusOverlayApi } from '../../ui/windowFocus';
import {
  ignoreAlert,
  type AlertHandler
} from '../../ui/dialogTypes';

export interface TriggerControllerOptions {
  addBackgroundTask: (text: string, options: { priority: BackgroundPriority }) => Task;
  alertUser?: AlertHandler;
  api?: FocusOverlayApi;
  captureUndoState?: (label: string) => void;
  findTaskById: (taskId: number) => Task | null;
  getTasks: () => Task[];
  onChanged: () => void;
}

export interface TriggerController {
  addNewBackgroundTaskFromTriggerModal: () => void;
  closeTriggersModal: () => void;
  configureTriggers: (taskId: number) => void;
  saveTriggers: () => void;
}

export function createTriggerController(options: TriggerControllerOptions): TriggerController {
  const alertUser = options.alertUser || ignoreAlert;
  let configuringTriggersForTaskId: number | null = null;

  function configureTriggers(taskId: number): void {
    configuringTriggersForTaskId = taskId;
    const task = options.findTaskById(taskId);
    if (!task) {
      return;
    }

    const backgroundTasks = flattenTasks(options.getTasks())
      .filter(currentTask => currentTask.mode === 'background');
    showTriggerConfigModal(task, backgroundTasks, options.api);
  }

  function closeTriggersModal(): void {
    closeTriggerConfigModal();
    configuringTriggersForTaskId = null;
  }

  function saveTriggers(): void {
    if (configuringTriggersForTaskId === null) {
      return;
    }

    const task = options.findTaskById(configuringTriggersForTaskId);
    if (!task) {
      return;
    }

    const nextTriggerIds = readSelectedTriggerIds();
    if (!areNumberArraysEqual(task.triggers, nextTriggerIds)) {
      options.captureUndoState?.('update task triggers');
      task.triggers = nextTriggerIds;
      options.onChanged();
    }
    closeTriggersModal();
  }

  function addNewBackgroundTaskFromTriggerModal(): void {
    const { text, highPriority } = readNewBackgroundTaskInput();
    if (!text) {
      void alertUser('Please enter a background task description.');
      return;
    }

    options.addBackgroundTask(text, {
      priority: highPriority ? 'high' : 'normal'
    });
    options.onChanged();

    if (configuringTriggersForTaskId !== null) {
      configureTriggers(configuringTriggersForTaskId);
    }
  }

  return {
    addNewBackgroundTaskFromTriggerModal,
    closeTriggersModal,
    configureTriggers,
    saveTriggers
  };
}

function areNumberArraysEqual(left: number[], right: number[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}
