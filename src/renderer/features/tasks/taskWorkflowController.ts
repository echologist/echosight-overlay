import type {
  Task,
  ThemeSoundEvent
} from '../../../shared/types';
import type { BackgroundTaskController } from './backgroundTaskController';
import {
  clearTaskInput,
  readTaskInput
} from './taskInputUi';
import {
  showCompletedTaskFeedback
} from './taskProgressUi';
import type { TaskStateController } from './taskStateController';
import { showReorderFeedback } from '../../ui/feedbackUi';
import {
  denyConfirm,
  ignoreAlert,
  type AlertHandler,
  type ConfirmHandler
} from '../../ui/dialogTypes';

type LogSink = Pick<Console, 'log' | 'error'>;
type ThemeSoundHandler = (event: ThemeSoundEvent) => void | Promise<void>;

export interface TaskWorkflowControllerOptions {
  alertUser?: AlertHandler;
  backgroundTasks: BackgroundTaskController;
  confirmUser?: ConfirmHandler;
  isInteractive: () => boolean;
  logger?: LogSink;
  playThemeSound?: ThemeSoundHandler;
  renderTasks: () => void;
  saveTasks: () => void | Promise<void>;
  taskState: TaskStateController;
  updateProgress: () => void;
}

export interface TaskWorkflowController {
  addBackgroundTask: (text: string, options?: unknown) => Task;
  addTask: (parentId?: number | null) => void;
  clearAllTasks: () => Promise<void>;
  completeNextTask: () => void;
  deleteTask: (taskId: number) => Promise<void>;
  findTaskById: (id: number, taskList?: Task[]) => Task | null;
  migrateTaskStructure: () => void;
  redoLastAction: () => void;
  reorderTasksAdvanced: (
    draggedId: number,
    targetId: number,
    insertAbove: boolean,
    makeSubtask?: boolean
  ) => void;
  restartExpirationTimers: () => void;
  toggleTask: (taskId: number) => void;
  undoLastAction: () => void;
}

export function createTaskWorkflowController(
  options: TaskWorkflowControllerOptions
): TaskWorkflowController {
  const logger = options.logger || console;
  const alertUser = options.alertUser || ignoreAlert;
  const confirmUser = options.confirmUser || denyConfirm;

  function persistTaskChanges(): void {
    options.renderTasks();
    options.updateProgress();
    void options.saveTasks();
  }

  function activateTriggeredTasks(task: Task): void {
    options.backgroundTasks.activateTriggeredTasks(task);
  }

  function deactivateTriggeredTasks(task: Task): void {
    options.backgroundTasks.deactivateTriggeredTasks(task);
  }

  function playThemeSound(event: ThemeSoundEvent): void {
    void options.playThemeSound?.(event);
  }

  function completeNextTask(): void {
    try {
      const result = options.taskState.completeNextTask();
      if (!result) {
        logger.log('No uncompleted leaf tasks found');
        return;
      }

      const { task, completedParents } = result;
      activateTriggeredTasks(task);
      completedParents.forEach(parent => {
        activateTriggeredTasks(parent);
      });

      persistTaskChanges();
      playThemeSound('taskCompleted');
      logger.log('Completed task:', task.text);

      if (options.isInteractive()) {
        showCompletedTaskFeedback(task.text);
      }
    } catch (error) {
      logger.error('Error completing next task:', error);
    }
  }

  function addTask(parentId: number | null = null): void {
    try {
      logger.log('addTask called with parentId:', parentId);
      const text = readTaskInput();
      logger.log('Task text:', text);

      if (!text) {
        logger.log('No text entered');
        return;
      }

      const result = options.taskState.addTask(text, parentId);
      if (!result) {
        logger.log('Parent task not found for new task:', parentId);
        return;
      }

      if (result.parentTask) {
        logger.log('Added subtask to parent:', result.parentTask.text);
      }

      clearTaskInput();
      persistTaskChanges();
      logger.log('Task added successfully');
    } catch (error) {
      logger.error('Error adding task:', error);
      void alertUser('Error adding task. Please try again.');
    }
  }

  function migrateTaskStructure(): void {
    const migrated = options.taskState.migrateTaskStructure();
    if (migrated) {
      void options.saveTasks();
    }
  }

  function addBackgroundTask(text: string, taskOptions: unknown = {}): Task {
    return options.taskState.addBackgroundTask(text, taskOptions);
  }

  function restartExpirationTimers(): void {
    options.backgroundTasks.restartExpirationTimers();
  }

  function toggleTask(taskId: number): void {
    const result = options.taskState.toggleTask(taskId);
    if (!result) {
      return;
    }

    if (result.task.completed) {
      activateTriggeredTasks(result.task);
      result.completedParents.forEach(parent => {
        activateTriggeredTasks(parent);
      });
      playThemeSound('taskCompleted');
    } else {
      deactivateTriggeredTasks(result.task);
      result.uncheckedParents.forEach(parent => {
        deactivateTriggeredTasks(parent);
      });
    }

    persistTaskChanges();
  }

  async function deleteTask(taskId: number): Promise<void> {
    const taskToDelete = options.taskState.findTaskById(taskId);
    if (!taskToDelete) {
      return;
    }

    if (!await confirmUser(`Delete "${taskToDelete.text}"? You can undo this with Ctrl+Shift+Z.`, {
      title: 'Delete Task',
      tone: 'danger'
    })) {
      return;
    }

    const result = options.taskState.deleteTask(taskId);
    result.removedTaskIds.forEach(id => {
      options.backgroundTasks.clearExpirationTimer(id);
    });

    if (result.removed) {
      persistTaskChanges();
    }
  }

  async function clearAllTasks(): Promise<void> {
    const totalTasks = options.taskState.getProgress().total;
    if (totalTasks === 0) {
      return;
    }

    if (!await confirmUser(`Clear all ${totalTasks} task${totalTasks === 1 ? '' : 's'}? You can undo this with Ctrl+Shift+Z.`, {
      title: 'Clear Tasks',
      tone: 'danger'
    })) {
      return;
    }

    options.backgroundTasks.clearAllExpirationTimers();
    options.taskState.clearTasks();
    persistTaskChanges();
  }

  function reorderTasksAdvanced(
    draggedId: number,
    targetId: number,
    insertAbove: boolean,
    makeSubtask = false
  ): void {
    logger.log('Advanced reordering:', { draggedId, targetId, insertAbove, makeSubtask });

    const result = options.taskState.moveTask(draggedId, targetId, insertAbove, makeSubtask);
    if (!result.success) {
      logger.log('Task reorder skipped:', result.reason);
      return;
    }

    logger.log('Operation completed:', result.operation);
    showReorderFeedback(result.operation);
    options.renderTasks();
    void options.saveTasks();
  }

  function undoLastAction(): void {
    const result = options.taskState.undoLastAction();
    if (!result.restored) {
      logger.log('No task action to undo');
      return;
    }

    options.backgroundTasks.clearAllExpirationTimers();
    options.backgroundTasks.restartExpirationTimers();
    persistTaskChanges();
    playThemeSound('undo');
    logger.log('Undid task action:', result.label);
  }

  function redoLastAction(): void {
    const result = options.taskState.redoLastAction();
    if (!result.restored) {
      logger.log('No task action to forward');
      return;
    }

    options.backgroundTasks.clearAllExpirationTimers();
    options.backgroundTasks.restartExpirationTimers();
    persistTaskChanges();
    playThemeSound('redo');
    logger.log('Forwarded task action:', result.label);
  }

  return {
    addBackgroundTask,
    addTask,
    clearAllTasks,
    completeNextTask,
    deleteTask,
    findTaskById: (id, taskList) => options.taskState.findTaskById(id, taskList),
    migrateTaskStructure,
    redoLastAction,
    reorderTasksAdvanced,
    restartExpirationTimers,
    toggleTask,
    undoLastAction
  };
}
