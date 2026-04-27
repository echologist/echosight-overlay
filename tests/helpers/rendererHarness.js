const fs = require('node:fs');
const path = require('node:path');
const { createTestDocument } = require('./fakeDom');
const { createFakeIpcRenderer } = require('./fakeIpc');

async function loadRendererModule(fileName) {
  const filePath = path.join(process.cwd(), 'src', 'renderer', fileName);
  const source = fs.readFileSync(filePath, 'utf8');
  const encoded = Buffer.from(`${source}\n//# sourceURL=${filePath}`).toString('base64');
  return import(`data:text/javascript;base64,${encoded}`);
}

function createDefaultState(overrides = {}) {
  return {
    tasks: overrides.tasks || [],
    snapshots: overrides.snapshots || [],
    templates: overrides.templates || [],
    themes: overrides.themes || [],
    currentTemplate: overrides.currentTemplate || null,
    isInteractiveMode: overrides.isInteractiveMode ?? true,
    settings: {
      transparency: 60,
      theme: 'echosight',
      hotkeys: {
        toggleVisibility: 'Ctrl+Shift+T',
        toggleInteractive: 'Ctrl+Shift+I',
        completeNextTask: 'Ctrl+Shift+N',
        undoLastAction: 'Ctrl+Shift+Z'
      },
      ...(overrides.settings || {}),
      hotkeys: {
        toggleVisibility: 'Ctrl+Shift+T',
        toggleInteractive: 'Ctrl+Shift+I',
        completeNextTask: 'Ctrl+Shift+N',
        undoLastAction: 'Ctrl+Shift+Z',
        ...((overrides.settings && overrides.settings.hotkeys) || {})
      }
    }
  };
}

async function createRendererHarness(options = {}) {
  const document = createTestDocument();
  const alerts = [];
  const confirms = [];
  const ipcRenderer = options.ipcRenderer || createFakeIpcRenderer(options.ipcResponses || {});
  const state = createDefaultState(options.state || {});
  const managers = {};
  const ctx = { ipcRenderer, state, managers };

  global.document = document;
  global.window = {
    focus() {},
    recordingHotkey: null
  };
  global.alert = message => alerts.push(message);
  global.confirm = message => {
    confirms.push(message);
    return options.confirmResult ?? true;
  };

  const [
    progressModule,
    themeModule,
    taskModule,
    templateModule,
    dragDropModule,
    uiModule
  ] = await Promise.all([
    loadRendererModule('progressTracker.js'),
    loadRendererModule('themeManager.js'),
    loadRendererModule('taskManager.js'),
    loadRendererModule('templateManager.js'),
    loadRendererModule('dragDrop.js'),
    loadRendererModule('uiRenderer.js')
  ]);

  managers.progress = progressModule.createProgressTracker(ctx);
  managers.theme = themeModule.createThemeManager(ctx);
  managers.task = taskModule.createTaskManager(ctx);
  managers.template = templateModule.createTemplateManager(ctx);
  managers.dragDrop = dragDropModule.createDragDrop(ctx);
  managers.ui = uiModule.createUiRenderer(ctx);

  return {
    alerts,
    confirms,
    ctx,
    document,
    ipcRenderer,
    managers,
    state,
    window: global.window
  };
}

async function closeViteServer() {
  return undefined;
}

module.exports = {
  closeViteServer,
  createRendererHarness
};
