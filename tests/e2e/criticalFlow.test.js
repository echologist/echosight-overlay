const { after, test } = require('node:test');
const assert = require('node:assert/strict');
const { closeViteServer, createRendererHarness } = require('../helpers/rendererHarness');

after(closeViteServer);

test('critical flow: add task, complete it, and persist the completed state', async () => {
  const savedPayloads = [];
  const { document, managers, state, window } = await createRendererHarness({
    ipcResponses: {
      'save-tasks': payload => {
        savedPayloads.push(JSON.parse(JSON.stringify(payload)));
        return { success: true };
      }
    }
  });

  window.addTask = managers.task.addTask;
  window.toggleTask = managers.task.toggleTask;

  document.getElementById('taskInput').value = 'Finish sanctum room';
  window.addTask();

  assert.equal(state.tasks.length, 1);
  assert.equal(document.querySelectorAll('.task-item').length, 1);
  assert.equal(document.getElementById('progressText').textContent, '0 / 1 tasks completed');

  window.toggleTask(state.tasks[0].id);

  assert.equal(state.tasks[0].completed, true);
  assert.equal(document.getElementById('progressFill').style.width, '100%');
  assert.equal(document.getElementById('progressText').textContent, '1 / 1 tasks completed');
  assert.equal(savedPayloads.length, 2);
  assert.equal(savedPayloads.at(-1).tasks[0].text, 'Finish sanctum room');
  assert.equal(savedPayloads.at(-1).tasks[0].completed, true);
});
