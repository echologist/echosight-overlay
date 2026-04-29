export type TaskMode = 'main' | 'background';
export type BackgroundPriority = 'normal' | 'high';

export interface BackgroundOptions {
  expiresAfterMinutes: number | null;
  priority: BackgroundPriority;
}

export interface Task {
  id: number;
  text: string;
  completed: boolean;
  createdAt: string;
  children: Task[];
  mode: TaskMode;
  triggers: number[];
  activated: boolean;
  activatedAt: string | null;
  backgroundOptions: BackgroundOptions | null;
}

export interface TemplateTask {
  text: string;
  children?: TemplateTask[];
  mode?: TaskMode;
  backgroundOptions?: BackgroundOptions | null;
  triggerIndices?: number[];
}

export interface TemplateData {
  name: string;
  tasks: TemplateTask[];
}

export interface TaskTemplate extends TemplateData {
  id: number;
  createdAt: string;
  imported?: boolean;
  community?: boolean;
}

export interface TemplateExportData extends TemplateData {
  version: '1.0' | '2.0';
  exportedAt: string;
  description: string;
  taskCount: number;
}

export interface TaskLocation {
  task: Task;
  parent: Task | null;
  level: number;
  array: Task[];
}

export interface HotkeySettings {
  toggleVisibility: string;
  toggleInteractive: string;
  completeNextTask: string;
  undoLastAction: string;
  redoLastAction: string;
}

export interface SoundSettings {
  enabled: boolean;
  volume: number;
}

export interface Settings {
  settingsVersion: number;
  transparency: number;
  theme: string;
  hotkeys: HotkeySettings;
  sounds: SoundSettings;
  backgroundColor?: string;
}

export interface TaskStateSnapshot {
  tasks: Task[];
  currentTemplate: string | null;
}

export interface TaskSnapshot {
  createdAt: string;
  state: TaskStateSnapshot;
}

export interface TaskSaveData extends TaskStateSnapshot {
  snapshots?: TaskSnapshot[];
}

export interface SaveResult {
  success: boolean;
  error?: string;
}

export interface ThemeAsset {
  path?: string;
  relativePath?: string;
  type: string;
  extension: string;
}

export interface ThemeAssetData {
  data: string;
  type: string;
  extension: string;
  mimeType: string;
  isText: boolean;
}

export type ThemeSoundEvent =
  'taskCompleted' |
  'backgroundActivated' |
  'undo' |
  'redo';

export interface ThemeSoundObject {
  file: string;
  volume?: number;
}

export type ThemeSoundDefinition = string | ThemeSoundObject;

export interface ThemeSoundConfig {
  enabled?: boolean;
  volume?: number;
  events?: Partial<Record<ThemeSoundEvent, ThemeSoundDefinition>>;
}

export interface Theme {
  id: string;
  name: string;
  description?: string;
  assets?: Record<string, ThemeAsset>;
  loadedAssets?: Record<string, string>;
  cssFile?: string;
  sounds?: ThemeSoundConfig;
  folderPath?: string;
  path?: string;
  [key: string]: unknown;
}

export type Unsubscribe = () => void;

export interface EchosightApi {
  loadTasks: () => Promise<TaskSaveData>;
  saveTasks: (tasksData: TaskSaveData) => Promise<SaveResult>;
  loadTemplates: () => Promise<unknown>;
  saveTemplates: (templates: TaskTemplate[]) => Promise<SaveResult>;
  loadSettings: () => Promise<Partial<Settings> | null>;
  saveSettings: (settings: Settings) => Promise<SaveResult>;

  loadThemes: () => Promise<Theme[]>;
  getTheme: (themeId: string) => Promise<Theme | null>;
  reloadThemes: () => Promise<Theme[]>;
  openThemesFolder: () => Promise<SaveResult>;
  getThemesPath: () => Promise<string>;
  loadThemeCss: (themeId: string, cssFileName: string) => Promise<string | null>;
  getThemeAsset: (themeId: string, assetName: string) => Promise<ThemeAssetData | null>;

  updateHotkeys: (hotkeys: HotkeySettings) => void;
  setHotkeyRecording: (recording: boolean) => void;
  focusWindow: () => void;
  resetWindowPosition: () => void;
  toggleOverlay: () => void;
  minimizeOverlay: () => void;
  toggleInteractiveMode: () => void;
  quitApplication: () => void;

  onInteractiveModeChanged: (callback: (interactive: boolean) => void) => Unsubscribe;
  onCompleteNextTask: (callback: () => void) => Unsubscribe;
  onUndoLastTaskAction: (callback: () => void) => Unsubscribe;
  onRedoLastTaskAction: (callback: () => void) => Unsubscribe;
}
