import type {
  EchosightApi,
  Task,
  TaskSnapshot,
  TaskStateSnapshot
} from '../../../shared/types';
import {
  completeNextTaskInList,
  toggleTaskInList,
  type CompleteTaskResult,
  type ToggleTaskResult
} from './taskCompletion';
import {
  createBackgroundTask
} from './taskCreation';
import {
  migrateTaskStructure as migrateTaskList
} from './taskMigration';
import {
  addTaskToList,
  deleteTaskFromList,
  type AddTaskResult,
  type DeleteTaskResult
} from './taskMutations';
import {
  loadTaskState,
  saveTaskState
} from './taskPersistence';
import {
  calculateTaskProgress,
  type TaskProgress
} from './taskProgress';
import { findTaskById } from './taskTree';
import {
  moveTaskInTree,
  type MoveTaskResult
} from './taskTreeOperations';

type LogSink = Pick<Console, 'log' | 'error'>;
const MAX_UNDO_ACTIONS = 20;
const MAX_AUTO_SAVE_SNAPSHOTS = 5;

export interface TaskStateControllerOptions {
  api: EchosightApi;
  logger?: LogSink;
}

export interface TaskStateController {
  addBackgroundTask: (text: string, options?: unknown) => Task;
  addTask: (text: string, parentId?: number | null) => AddTaskResult | null;
  captureUndoState: (label: string) => void;
  clearTasks: () => void;
  completeNextTask: () => CompleteTaskResult | null;
  deleteTask: (taskId: number) => DeleteTaskResult;
  findTaskById: (id: number, taskList?: Task[]) => Task | null;
  getProgress: () => TaskProgress;
  getTasks: () => Task[];
  loadTasks: () => Promise<void>;
  migrateTaskStructure: () => boolean;
  moveTask: (draggedId: number, targetId: number, insertAbove: boolean, makeSubtask?: boolean) => MoveTaskResult;
  replaceTasks: (nextTasks: Task[], nextCurrentTemplate: string | null) => void;
  redoLastAction: () => UndoTaskActionResult;
  saveTasks: () => Promise<void>;
  toggleTask: (taskId: number) => ToggleTaskResult | null;
  undoLastAction: () => UndoTaskActionResult;
}

export interface UndoTaskActionResult {
  restored: boolean;
  label?: string;
}

interface UndoEntry {
  label: string;
  state: TaskStateSnapshot;
  createdAt: string;
}

export function createTaskStateController(options: TaskStateControllerOptions): TaskStateController {
  const logger = options.logger || console;
  let tasks: Task[] = [];
  let currentTemplate: string | null = null;
  let snapshots: TaskSnapshot[] = [];
  const undoStack: UndoEntry[] = [];
  const redoStack: UndoEntry[] = [];

  function getTasks(): Task[] {
    return tasks;
  }

  function replaceTasks(nextTasks: Task[], nextCurrentTemplate: string | null): void {
    tasks = nextTasks;
    currentTemplate = nextCurrentTemplate;
  }

  async function loadTasks(): Promise<void> {
    const taskState = await loadTaskState(options.api, logger);
    replaceTasks(taskState.tasks, taskState.currentTemplate);
    snapshots = taskState.snapshots.slice(-MAX_AUTO_SAVE_SNAPSHOTS);
  }

  async function saveTasks(): Promise<void> {
    recordAutoSaveSnapshot();
    await saveTaskState(options.api, {
      tasks,
      currentTemplate,
      snapshots
    }, logger);
  }

  function migrateTaskStructure(): boolean {
    const migrated = migrateTaskList(tasks);
    if (migrated) {
      logger.log('Tasks migrated to support background task structure');
    }

    return migrated;
  }

  function addBackgroundTask(text: string, options: unknown = {}): Task {
    return withUndo('add background task', () => {
      const task = createBackgroundTask(text, options);
      tasks.push(task);
      return task;
    });
  }

  function captureUndoState(label: string): void {
    pushUndoState(label, cloneTaskState());
  }

  function clearTasks(): void {
    if (tasks.length === 0 && currentTemplate === null) {
      return;
    }

    captureUndoState('clear all tasks');
    replaceTasks([], null);
  }

  function undoLastAction(): UndoTaskActionResult {
    const undoEntry = undoStack.pop();
    if (!undoEntry) {
      return { restored: false };
    }

    pushRedoState(undoEntry.label, cloneTaskState());
    restoreTaskState(undoEntry.state);
    return {
      restored: true,
      label: undoEntry.label
    };
  }

  function redoLastAction(): UndoTaskActionResult {
    const redoEntry = redoStack.pop();
    if (!redoEntry) {
      return { restored: false };
    }

    pushUndoStateForHistory(redoEntry.label, cloneTaskState());
    restoreTaskState(redoEntry.state);
    return {
      restored: true,
      label: redoEntry.label
    };
  }

  function withUndo<Result>(
    label: string,
    mutate: () => Result,
    didMutate: (result: Result) => boolean = () => true
  ): Result {
    const previousState = cloneTaskState();
    const result = mutate();
    if (didMutate(result)) {
      pushUndoState(label, previousState);
    }
    return result;
  }

  function pushUndoState(label: string, state: TaskStateSnapshot): void {
    pushUndoStateForHistory(label, state);
    redoStack.length = 0;
  }

  function pushUndoStateForHistory(label: string, state: TaskStateSnapshot): void {
    pushHistoryEntry(undoStack, label, state);
  }

  function pushRedoState(label: string, state: TaskStateSnapshot): void {
    pushHistoryEntry(redoStack, label, state);
  }

  function pushHistoryEntry(stack: UndoEntry[], label: string, state: TaskStateSnapshot): void {
    stack.push({
      label,
      state,
      createdAt: new Date().toISOString()
    });

    if (stack.length > MAX_UNDO_ACTIONS) {
      stack.shift();
    }
  }

  function cloneTaskState(): TaskStateSnapshot {
    return {
      tasks: cloneTasks(tasks),
      currentTemplate
    };
  }

  function restoreTaskState(snapshot: TaskStateSnapshot): void {
    tasks = cloneTasks(snapshot.tasks);
    currentTemplate = snapshot.currentTemplate;
  }

  function recordAutoSaveSnapshot(): void {
    snapshots = [
      ...snapshots,
      {
        createdAt: new Date().toISOString(),
        state: cloneTaskState()
      }
    ].slice(-MAX_AUTO_SAVE_SNAPSHOTS);
  }

  return {
    addBackgroundTask,
    addTask: (text, parentId = null) =>
      withUndo('add task', () => addTaskToList(tasks, text, parentId), result => result !== null),
    captureUndoState,
    clearTasks,
    completeNextTask: () =>
      withUndo('complete next task', () => completeNextTaskInList(tasks), result => result !== null),
    deleteTask: taskId =>
      withUndo('delete task', () => deleteTaskFromList(taskId, tasks), result => result.removed),
    findTaskById: (id, taskList = tasks) => findTaskById(id, taskList),
    getProgress: () => calculateTaskProgress(tasks),
    getTasks,
    loadTasks,
    migrateTaskStructure,
    moveTask: (draggedId, targetId, insertAbove, makeSubtask = false) =>
      withUndo(
        'move task',
        () => moveTaskInTree(tasks, draggedId, targetId, insertAbove, makeSubtask),
        result => result.success
      ),
    replaceTasks,
    redoLastAction,
    saveTasks,
    toggleTask: taskId =>
      withUndo('toggle task', () => toggleTaskInList(taskId, tasks), result => result !== null),
    undoLastAction
  };
}

function cloneTasks(tasks: Task[]): Task[] {
  return JSON.parse(JSON.stringify(tasks)) as Task[];
}
