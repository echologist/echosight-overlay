const { after, test } = require('node:test');
const assert = require('node:assert/strict');
const { closeViteServer, createRendererHarness } = require('../helpers/rendererHarness');

after(closeViteServer);

test('addTask creates a top-level task and persists it', async () => {
  const { document, ipcRenderer, managers, state } = await createRendererHarness();

  document.getElementById('taskInput').value = 'Run third lab';
  managers.task.addTask();

  assert.equal(state.tasks.length, 1);
  assert.equal(state.tasks[0].text, 'Run third lab');
  assert.equal(state.tasks[0].completed, false);
  assert.equal(state.tasks[0].mode, 'main');
  assert.equal(document.getElementById('taskInput').value, '');
  assert.equal(ipcRenderer.calls('save-tasks').length, 1);
});

test('deleteTask removes a task and cleans trigger references', async () => {
  const backgroundTask = {
    id: 20,
    text: 'Use omen',
    completed: false,
    children: [],
    mode: 'background',
    triggers: [],
    activated: false,
    activatedAt: null,
    backgroundOptions: null
  };

  const { ipcRenderer, managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 10,
          text: 'Kill boss',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [backgroundTask.id],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        },
        backgroundTask
      ]
    }
  });

  managers.task.deleteTask(backgroundTask.id);

  assert.deepEqual(state.tasks.map(task => task.id), [10]);
  assert.deepEqual(state.tasks[0].triggers, []);
  assert.equal(ipcRenderer.calls('save-tasks').length, 1);
});

test('completing a parent cascades completion to all descendants', async () => {
  const { managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 1,
          text: 'Atlas setup',
          completed: false,
          children: [
            { id: 2, text: 'Buy maps', completed: false, children: [], mode: 'main', triggers: [] },
            { id: 3, text: 'Roll maps', completed: false, children: [], mode: 'main', triggers: [] }
          ],
          mode: 'main',
          triggers: [],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        }
      ]
    }
  });

  managers.task.toggleTask(1);

  assert.equal(state.tasks[0].completed, true);
  assert.equal(state.tasks[0].children[0].completed, true);
  assert.equal(state.tasks[0].children[1].completed, true);
});

test('completing the final child auto-completes the parent', async () => {
  const { managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 1,
          text: 'Campaign',
          completed: false,
          children: [
            { id: 2, text: 'Act 1', completed: true, children: [], mode: 'main', triggers: [] },
            { id: 3, text: 'Act 2', completed: false, children: [], mode: 'main', triggers: [] }
          ],
          mode: 'main',
          triggers: [],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        }
      ]
    }
  });

  managers.task.toggleTask(3);

  assert.equal(state.tasks[0].completed, true);
});

test('task completion activates and uncompletion deactivates linked background tasks', async () => {
  const originalSetTimeout = global.setTimeout;
  const { managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 1,
          text: 'Enter map',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [99],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        },
        {
          id: 99,
          text: 'Check altar mods',
          completed: false,
          children: [],
          mode: 'background',
          triggers: [],
          activated: false,
          activatedAt: null,
          backgroundOptions: { priority: 'high', expiresAfterMinutes: null }
        }
      ]
    }
  });

  global.setTimeout = callback => {
    callback();
    return 0;
  };

  try {
    managers.task.toggleTask(1);

    assert.equal(state.tasks[1].activated, true);
    assert.match(state.tasks[1].activatedAt, /^\d{4}-\d{2}-\d{2}T/);

    managers.task.toggleTask(1);

    assert.equal(state.tasks[1].activated, false);
    assert.equal(state.tasks[1].activatedAt, null);
  } finally {
    global.setTimeout = originalSetTimeout;
  }
});

test('completeNextTask completes an activated background task after main tasks are done', async () => {
  const { managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 1,
          text: 'Finish campaign step',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [99],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        },
        {
          id: 99,
          text: 'Check spawned objective',
          completed: false,
          children: [],
          mode: 'background',
          triggers: [],
          activated: false,
          activatedAt: null,
          backgroundOptions: { priority: 'normal', expiresAfterMinutes: null }
        }
      ]
    }
  });

  managers.task.completeNextTask();
  assert.equal(state.tasks[0].completed, true);
  assert.equal(state.tasks[1].activated, true);
  assert.equal(state.tasks[1].completed, false);

  managers.task.completeNextTask();

  assert.equal(state.tasks[1].completed, true);
});

test('toggleTask can be undone and persists restored state', async () => {
  const { ipcRenderer, managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 1,
          text: 'Act 1',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        }
      ]
    }
  });

  managers.task.toggleTask(1);
  managers.task.undoLastAction();

  assert.equal(state.tasks[0].completed, false);
  assert.equal(ipcRenderer.calls('save-tasks').at(-1).args[0].tasks[0].completed, false);
});

test('deleteTask asks for confirmation and cancel preserves state', async () => {
  const { confirms, ipcRenderer, managers, state } = await createRendererHarness({
    confirmResult: false,
    state: {
      tasks: [
        {
          id: 1,
          text: 'Do not delete',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        }
      ]
    }
  });

  managers.task.deleteTask(1);

  assert.equal(confirms.length, 1);
  assert.match(confirms[0], /Do not delete/);
  assert.deepEqual(state.tasks.map(task => task.id), [1]);
  assert.equal(ipcRenderer.calls('save-tasks').length, 0);
});

test('confirmed delete can be undone including trigger references', async () => {
  const backgroundTask = {
    id: 20,
    text: 'Use omen',
    completed: false,
    children: [],
    mode: 'background',
    triggers: [],
    activated: false,
    activatedAt: null,
    backgroundOptions: null
  };

  const { managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 10,
          text: 'Kill boss',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [backgroundTask.id],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        },
        backgroundTask
      ]
    }
  });

  managers.task.deleteTask(backgroundTask.id);
  managers.task.undoLastAction();

  assert.deepEqual(state.tasks.map(task => task.id), [10, 20]);
  assert.deepEqual(state.tasks[0].triggers, [20]);
});

test('clearAllTasks can be undone', async () => {
  const { managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 1,
          text: 'Keep me',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        }
      ]
    }
  });

  managers.task.clearAllTasks();
  assert.equal(state.tasks.length, 0);

  managers.task.undoLastAction();

  assert.equal(state.tasks.length, 1);
  assert.equal(state.tasks[0].text, 'Keep me');
});

test('saveTasks persists only the last five auto-save snapshots', async () => {
  const { ipcRenderer, managers } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 1,
          text: 'Snapshot target',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        }
      ]
    }
  });

  for (let i = 0; i < 6; i++) {
    await managers.task.saveTasks();
  }

  const payload = ipcRenderer.calls('save-tasks').at(-1).args[0];
  assert.equal(payload.snapshots.length, 5);
  assert.equal(payload.snapshots.at(-1).state.tasks[0].text, 'Snapshot target');
});

test('undo-last-task-action IPC event restores the previous task state', async () => {
  const { ipcRenderer, managers, state } = await createRendererHarness({
    state: {
      tasks: [
        {
          id: 1,
          text: 'Misclick',
          completed: false,
          children: [],
          mode: 'main',
          triggers: [],
          activated: true,
          activatedAt: null,
          backgroundOptions: null
        }
      ]
    }
  });

  managers.task.setupInteractiveModeListener();
  managers.task.toggleTask(1);
  ipcRenderer.emit('undo-last-task-action');

  assert.equal(state.tasks[0].completed, false);
});
