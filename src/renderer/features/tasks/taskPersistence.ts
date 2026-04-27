import type {
  EchosightApi,
  Task,
  TaskSaveData,
  TaskSnapshot
} from '../../../shared/types';

export interface LoadedTaskState {
  tasks: Task[];
  currentTemplate: string | null;
  snapshots: TaskSnapshot[];
}

export async function loadTaskState(api: EchosightApi, logger: Pick<Console, 'error'> = console): Promise<LoadedTaskState> {
  try {
    const data = await api.loadTasks();
    return normalizeTaskSaveData(data);
  } catch (error) {
    logger.error('Failed to load tasks:', error);
    return { tasks: [], currentTemplate: null, snapshots: [] };
  }
}

export async function saveTaskState(
  api: EchosightApi,
  taskState: TaskSaveData,
  logger: Pick<Console, 'error'> = console
): Promise<void> {
  try {
    const result = await api.saveTasks(taskState);
    if (!result.success) {
      throw new Error(result.error || 'Unknown save error');
    }
  } catch (error) {
    logger.error('Failed to save tasks:', error);
  }
}

function normalizeTaskSaveData(data: TaskSaveData | null | undefined): LoadedTaskState {
  return {
    tasks: Array.isArray(data?.tasks) ? data.tasks : [],
    currentTemplate: typeof data?.currentTemplate === 'string' ? data.currentTemplate : null,
    snapshots: Array.isArray(data?.snapshots) ? data.snapshots.slice(-5) : []
  };
}
