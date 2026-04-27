const { after, test } = require('node:test');
const assert = require('node:assert/strict');
const { closeViteServer, createRendererHarness } = require('../helpers/rendererHarness');

after(closeViteServer);

test('task manager loads and saves tasks over the expected IPC channels', async () => {
  const { ipcRenderer, managers, state } = await createRendererHarness({
    ipcResponses: {
      'load-tasks': {
        currentTemplate: 'Mapping',
        tasks: [
          { id: 1, text: 'Open map', completed: false, children: [], mode: 'main', triggers: [] }
        ]
      },
      'save-tasks': { success: true }
    }
  });

  await managers.task.loadTasks();
  assert.deepEqual(state.snapshots, []);

  state.tasks[0].completed = true;
  await managers.task.saveTasks();

  assert.equal(ipcRenderer.calls('load-tasks').length, 1);
  assert.equal(ipcRenderer.calls('save-tasks').length, 1);

  const savePayload = ipcRenderer.calls('save-tasks')[0].args[0];
  assert.deepEqual(savePayload.tasks, [
    { id: 1, text: 'Open map', completed: true, children: [], mode: 'main', triggers: [] }
  ]);
  assert.equal(savePayload.currentTemplate, 'Mapping');
  assert.equal(savePayload.snapshots.length, 1);
  assert.match(savePayload.snapshots[0].createdAt, /^\d{4}-\d{2}-\d{2}T/);
  assert.deepEqual(savePayload.snapshots[0].state, {
    currentTemplate: 'Mapping',
    tasks: [
      { id: 1, text: 'Open map', completed: true, children: [], mode: 'main', triggers: [] }
    ]
  });
});

test('template manager loads and saves templates over IPC', async () => {
  const { ipcRenderer, managers, state } = await createRendererHarness({
    ipcResponses: {
      'load-templates': [
        { id: 1, name: 'Bossing', tasks: [{ text: 'Set pantheon', children: [] }] }
      ],
      'save-templates': { success: true }
    }
  });

  await managers.template.loadTemplates();
  state.templates.push({ id: 2, name: 'Mapping', tasks: [{ text: 'Roll maps', children: [] }] });
  await managers.template.saveTemplates();

  assert.equal(ipcRenderer.calls('load-templates').length, 1);
  assert.equal(ipcRenderer.calls('save-templates').length, 1);
  assert.equal(ipcRenderer.calls('save-templates')[0].args[0].length, 2);
});

test('theme manager loads themes, merges settings, saves settings, and sends hotkey updates', async () => {
  const { document, ipcRenderer, managers, state } = await createRendererHarness({
    ipcResponses: {
      'load-themes': [{ id: 'dark', name: 'Dark' }],
      'load-settings': {
        transparency: 75,
        theme: 'dark',
        hotkeys: { completeNextTask: 'Alt+N', undoLastAction: 'Alt+Z' }
      },
      'save-settings': { success: true }
    }
  });

  await managers.theme.loadThemes();
  await managers.theme.loadSettings();

  assert.deepEqual(state.themes, [{ id: 'dark', name: 'Dark' }]);
  assert.equal(state.settings.transparency, 75);
  assert.equal(state.settings.hotkeys.toggleVisibility, 'Ctrl+Shift+T');
  assert.equal(state.settings.hotkeys.completeNextTask, 'Alt+N');
  assert.equal(state.settings.hotkeys.undoLastAction, 'Alt+Z');

  document.getElementById('transparencySlider').value = '80';
  document.getElementById('themeSelect').value = 'dark';
  document.getElementById('toggleVisibilityHotkey').value = 'Ctrl+F1';
  document.getElementById('toggleInteractiveHotkey').value = 'Ctrl+F2';
  document.getElementById('completeNextTaskHotkey').value = 'Ctrl+F3';
  document.getElementById('undoLastActionHotkey').value = 'Ctrl+F4';

  await managers.theme.saveSettings();

  assert.equal(ipcRenderer.calls('save-settings').length, 1);
  assert.deepEqual(ipcRenderer.calls('save-settings')[0].args[0].hotkeys, {
    toggleVisibility: 'Ctrl+F1',
    toggleInteractive: 'Ctrl+F2',
    completeNextTask: 'Ctrl+F3',
    undoLastAction: 'Ctrl+F4'
  });
  assert.deepEqual(ipcRenderer.sent('update-hotkeys')[0].args[0], {
    toggleVisibility: 'Ctrl+F1',
    toggleInteractive: 'Ctrl+F2',
    completeNextTask: 'Ctrl+F3',
    undoLastAction: 'Ctrl+F4'
  });
});
