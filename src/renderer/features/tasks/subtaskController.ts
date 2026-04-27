import type { Task } from '../../../shared/types';
import type { FocusOverlayApi } from '../../ui/windowFocus';
import type { AddTaskResult } from './taskMutations';
import {
  closeSubTaskModal as closeSubTaskModalUi,
  readSubTaskInput,
  showSubTaskModal
} from './subtaskUi';
import {
  ignoreAlert,
  type AlertHandler
} from '../../ui/dialogTypes';

type LogSink = Pick<Console, 'log'>;

export interface SubtaskControllerOptions {
  api?: FocusOverlayApi;
  addTask: (text: string, parentId: number) => AddTaskResult | null;
  alertUser?: AlertHandler;
  getTasks: () => Task[];
  logger?: LogSink;
  onChanged: () => void;
}

export interface SubtaskController {
  addSubTask: (parentId: number) => void;
  closeSubTaskModal: () => void;
  saveSubTask: () => void;
}

export function createSubtaskController(options: SubtaskControllerOptions): SubtaskController {
  const logger = options.logger || console;
  const alertUser = options.alertUser || ignoreAlert;
  let currentParentId: number | null = null;

  function addSubTask(parentId: number): void {
    logger.log('addSubTask called with parentId:', parentId);
    currentParentId = parentId;
    showSubTaskModal(options.api);
  }

  function closeSubTaskModal(): void {
    closeSubTaskModalUi(options.api);
    currentParentId = null;
  }

  function saveSubTask(): void {
    const taskText = readSubTaskInput();
    logger.log('User entered:', taskText);

    if (!taskText) {
      void alertUser('Please enter a task name!');
      return;
    }

    if (currentParentId === null) {
      return;
    }

    const result = options.addTask(taskText, currentParentId);
    logger.log('Found parent task:', result?.parentTask || null);

    if (!result?.parentTask) {
      return;
    }

    logger.log('Added subtask, parent now has:', result.parentTask.children.length, 'children');
    options.onChanged();
    closeSubTaskModal();
  }

  return {
    addSubTask,
    closeSubTaskModal,
    saveSubTask
  };
}
