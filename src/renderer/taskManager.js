export function createTaskManager(ctx) {
  const { ipcRenderer, state } = ctx;
  const { tasks } = state;
  const MAX_UNDO_ACTIONS = 20;
  const MAX_AUTO_SAVE_SNAPSHOTS = 5;
  let undoStack = [];

  if (!Array.isArray(state.snapshots)) {
    state.snapshots = [];
  }

  function renderTasks() {
    ctx.managers.ui.renderTasks();
  }

  function updateProgress() {
    ctx.managers.progress.updateProgress();
  }

  function updateInteractiveVisuals(interactive) {
    ctx.managers.ui.updateInteractiveVisuals(interactive);
  }

  function escapeHtml(text) {
    return ctx.managers.ui.escapeHtml(text);
  }

  function cloneTaskState() {
    return JSON.parse(JSON.stringify({
      tasks,
      currentTemplate: state.currentTemplate
    }));
  }

  function pushUndoState(label) {
    undoStack.push({
      label,
      state: cloneTaskState(),
      createdAt: new Date().toISOString()
    });

    if (undoStack.length > MAX_UNDO_ACTIONS) {
      undoStack.shift();
    }
  }

  function restoreTaskState(snapshot) {
    tasks.splice(0, tasks.length, ...(snapshot.tasks || []));
    state.currentTemplate = snapshot.currentTemplate || null;
  }

  function recordAutoSaveSnapshot() {
    state.snapshots.push({
      createdAt: new Date().toISOString(),
      state: cloneTaskState()
    });

    state.snapshots = state.snapshots.slice(-MAX_AUTO_SAVE_SNAPSHOTS);
  }

  function countAllTasks(taskList = tasks) {
    return taskList.reduce((count, task) => {
      return count + 1 + countAllTasks(task.children || []);
    }, 0);
  }

  function undoLastAction() {
    const undoEntry = undoStack.pop();
    if (!undoEntry) {
      console.log('No task action to undo');
      return false;
    }

    restoreTaskState(undoEntry.state);
    syncExpirationTimers();
    renderTasks();
    updateProgress();
    saveTasks();
    console.log('Undid task action:', undoEntry.label);
    return true;
  }

// Handle interactive mode changes from main process
function setupInteractiveModeListener() {
  ipcRenderer.on('interactive-mode-changed', (event, interactive) => {
    console.log('Interactive mode changed:', interactive);
    state.isInteractiveMode = interactive;
    updateInteractiveVisuals(interactive);
  });

  // Handle complete next task from main process
  ipcRenderer.on('complete-next-task', () => {
    console.log('Complete next task triggered');
    completeNextTask();
  });

  ipcRenderer.on('undo-last-task-action', () => {
    console.log('Undo last task action triggered');
    undoLastAction();
  });
}

// Complete the next uncompleted task - main tasks first, then active background tasks
function completeNextTask() {
  try {
    const nextTask = findNextLeafTask();
    if (nextTask) {
      pushUndoState('complete next task');
      nextTask.completed = true;

      // Check if parent should auto-complete
      checkParentCompletion(nextTask.id);

      // Fire triggers for the completed task
      activateTriggeredTasks(nextTask);
      // Also check if parent was auto-completed and fire its triggers
      const parent = findParentTask(nextTask.id);
      if (parent && parent.completed) {
        activateTriggeredTasks(parent);
      }

      renderTasks();
      updateProgress();
      saveTasks();
      console.log('Completed task:', nextTask.text);

      // Optional: Show a brief visual feedback
      if (state.isInteractiveMode) {
        const progressText = document.getElementById('progressText');
        const originalText = progressText.textContent;
        progressText.textContent = `✓ Completed: ${nextTask.text}`;
        progressText.style.color = '#32cd32';
        setTimeout(() => {
          progressText.textContent = originalText;
          progressText.style.color = '';
        }, 2000);
      }
    } else {
      console.log('No uncompleted leaf tasks found');
    }
  } catch (error) {
    console.error('Error completing next task:', error);
  }
}

// Find next uncompleted leaf task. Top-level calls prioritize main tasks, then active background tasks.
function findNextLeafTask(taskList = null) {
  if (taskList === null) {
    return findNextLeafTask(tasks.filter(t => t.mode !== 'background'))
      || findNextLeafTask(tasks.filter(t => t.mode === 'background' && t.activated));
  }
  for (const task of taskList) {
    if (!task.completed) {
      // If task has children, it's a parent - skip it and check children
      if (task.children && task.children.length > 0) {
        const leafTask = findNextLeafTask(task.children);
        if (leafTask) return leafTask;
      } else {
        // It's a leaf task (no children)
        return task;
      }
    }
  }
  return null;
}


// Task management
function addTask(parentId = null) {
  try {
    console.log('addTask called with parentId:', parentId);
    const input = document.getElementById('taskInput');
    const text = input.value.trim();
    console.log('Task text:', text);

    if (text) {
      const task = {
        id: Date.now(),
        text: text,
        completed: false,
        createdAt: new Date().toISOString(),
        children: [],
        mode: 'main',
        triggers: [],
        activated: true,
        activatedAt: null,
        backgroundOptions: null
      };

      if (parentId) {
        // Find parent task and add as child
        const parentTask = findTaskById(parentId);
        if (parentTask) {
          parentTask.children.push(task);
          console.log('Added subtask to parent:', parentTask.text);
        }
      } else {
        // Add as top-level task
        tasks.push(task);
      }

      input.value = '';
      renderTasks();
      updateProgress();
      saveTasks();
      console.log('Task added successfully');
    } else {
      console.log('No text entered');
    }
  } catch (error) {
    console.error('Error adding task:', error);
    alert('Error adding task. Please try again.');
  }
}

// Migration function to ensure all tasks have required fields
function migrateTaskStructure() {
  let migrated = false;
  function migrateTask(task) {
    if (!task.children) { task.children = []; migrated = true; }
    if (!task.mode) { task.mode = 'main'; migrated = true; }
    if (!task.triggers) { task.triggers = []; migrated = true; }
    if (task.activated === undefined) { task.activated = true; migrated = true; }
    if (task.activatedAt === undefined) { task.activatedAt = null; migrated = true; }
    if (task.backgroundOptions === undefined) { task.backgroundOptions = null; migrated = true; }
    task.children.forEach(migrateTask);
  }
  tasks.forEach(migrateTask);
  if (migrated) {
    console.log('Tasks migrated to support background task structure');
    saveTasks();
  }
}

// Helper function to find task by ID (recursive)
function findTaskById(id, taskList = tasks) {
  for (const task of taskList) {
    if (task.id === id) {
      return task;
    }
    if (task.children && task.children.length > 0) {
      const found = findTaskById(id, task.children);
      if (found) return found;
    }
  }
  return null;
}

// Complete all children of a parent task
function completeAllChildren(parentTask) {
  if (parentTask.children) {
    parentTask.children.forEach(child => {
      child.completed = true;
      if (child.children && child.children.length > 0) {
        completeAllChildren(child);
      }
    });
  }
}

// Check if parent should auto-complete when all children are done
function checkParentCompletion(childId) {
  const parentTask = findParentTask(childId);
  if (parentTask && parentTask.children.length > 0) {
    const allChildrenComplete = parentTask.children.every(child => child.completed);
    if (allChildrenComplete && !parentTask.completed) {
      parentTask.completed = true;
    }
  }
}

// Uncheck parent when child is unchecked
function uncheckParent(childId) {
  const parentTask = findParentTask(childId);
  if (parentTask && parentTask.completed) {
    parentTask.completed = false;
  }
}

// Activate background tasks triggered by a completed task
function activateTriggeredTasks(task) {
  if (!task.triggers || task.triggers.length === 0) return;

  let activatedCount = 0;
  task.triggers.forEach(triggeredId => {
    const bgTask = findTaskById(triggeredId);
    if (bgTask && bgTask.mode === 'background' && !bgTask.activated) {
      bgTask.activated = true;
      bgTask.activatedAt = new Date().toISOString();
      activatedCount++;

      // Start expiration timer if configured
      if (bgTask.backgroundOptions?.expiresAfterMinutes) {
        startExpirationTimer(bgTask);
      }
    }
  });

  // Show notification if background tasks were activated
  if (activatedCount > 0) {
    showBackgroundTaskNotification(activatedCount);
  }
}

// Deactivate background tasks when their trigger task is un-completed
function deactivateTriggeredTasks(task) {
  if (!task.triggers || task.triggers.length === 0) return;

  task.triggers.forEach(triggeredId => {
    const bgTask = findTaskById(triggeredId);
    if (bgTask && bgTask.mode === 'background' && bgTask.activated && !bgTask.completed) {
      bgTask.activated = false;
      bgTask.activatedAt = null;
      // Clear any running expiration timer
      if (expirationTimers[bgTask.id]) {
        clearTimeout(expirationTimers[bgTask.id]);
        delete expirationTimers[bgTask.id];
      }
    }
  });
}

// Notification when background tasks activate
function showBackgroundTaskNotification(count) {
  const notification = document.createElement('div');
  notification.className = 'bg-task-notification';
  notification.textContent = `${count} background task${count > 1 ? 's' : ''} activated!`;
  document.body.appendChild(notification);
  setTimeout(() => {
    if (notification.parentNode) {
      notification.style.animation = 'slideIn 0.3s ease reverse';
      setTimeout(() => notification.remove(), 300);
    }
  }, 3000);
}

// Expiration timers for background tasks
const expirationTimers = {};

function syncExpirationTimers() {
  Object.keys(expirationTimers).forEach(taskId => {
    clearTimeout(expirationTimers[taskId]);
    delete expirationTimers[taskId];
  });

  restartExpirationTimers();
}

function startExpirationTimer(bgTask) {
  if (expirationTimers[bgTask.id]) {
    clearTimeout(expirationTimers[bgTask.id]);
  }

  let ms;
  if (bgTask.activatedAt) {
    // Calculate remaining time (for restart scenarios)
    const elapsed = Date.now() - new Date(bgTask.activatedAt).getTime();
    ms = (bgTask.backgroundOptions.expiresAfterMinutes * 60 * 1000) - elapsed;
    if (ms <= 0) {
      // Already expired
      bgTask.activated = false;
      renderTasks();
      updateProgress();
      saveTasks();
      return;
    }
  } else {
    ms = bgTask.backgroundOptions.expiresAfterMinutes * 60 * 1000;
  }

  expirationTimers[bgTask.id] = setTimeout(() => {
    if (bgTask.activated && !bgTask.completed) {
      bgTask.activated = false;
      renderTasks();
      updateProgress();
      saveTasks();
      console.log(`Background task "${bgTask.text}" expired`);
    }
    delete expirationTimers[bgTask.id];
  }, ms);
}

// Dismiss a background task (deactivate without deleting)
function dismissBackgroundTask(taskId) {
  // Background tasks are removed via the standard delete button
  deleteTask(taskId);
}

// Create a new background task (dormant until triggered)
function addBackgroundTask(text, options = {}) {
  const task = {
    id: Date.now() + Math.random(),
    text: text,
    completed: false,
    createdAt: new Date().toISOString(),
    children: [],
    mode: 'background',
    triggers: [],
    activated: false,
    activatedAt: null,
    backgroundOptions: {
      expiresAfterMinutes: options.expiresAfterMinutes || null,
      priority: options.priority || 'normal'
    }
  };
  tasks.push(task);
  return task;
}

// Trigger configuration UI
let configuringTriggersForTaskId = null;

function configureTriggers(taskId) {
  configuringTriggersForTaskId = taskId;
  const task = findTaskById(taskId);
  if (!task) return;

  const modal = document.getElementById('configureTriggersModal');
  const listContainer = document.getElementById('triggerTaskList');
  listContainer.innerHTML = '';

  // Show all background tasks as checkboxes
  const bgTasks = tasks.filter(t => t.mode === 'background');

  if (bgTasks.length === 0) {
    listContainer.innerHTML = '<p style="color: #888; font-size: 12px;">No background tasks yet. Create one below.</p>';
  } else {
    bgTasks.forEach(bgTask => {
      const isLinked = task.triggers.includes(bgTask.id);
      const div = document.createElement('div');
      div.style.cssText = 'padding: 6px 0; display: flex; align-items: center; border-bottom: 1px solid #333;';
      const priorityBadge = bgTask.backgroundOptions?.priority === 'high'
        ? '<span style="color: #ff6b6b; font-size: 9px; margin-left: 4px;">HIGH</span>'
        : '';
      const statusBadge = bgTask.activated
        ? '<span style="color: #32cd32; font-size: 9px; margin-left: 4px;">ACTIVE</span>'
        : '<span style="color: #666; font-size: 9px; margin-left: 4px;">DORMANT</span>';
      div.innerHTML = `
        <input type="checkbox" ${isLinked ? 'checked' : ''}
               data-bg-task-id="${bgTask.id}" class="trigger-checkbox"
               style="accent-color: #d4af37; margin-right: 8px;">
        <span style="font-size: 13px; flex: 1;">${escapeHtml(bgTask.text)}</span>
        ${priorityBadge}${statusBadge}
      `;
      listContainer.appendChild(div);
    });
  }

  // Clear the new background task input
  const bgInput = document.getElementById('newBgTaskInput');
  if (bgInput) bgInput.value = '';

  modal.style.display = 'flex';
}

function closeTriggersModal() {
  document.getElementById('configureTriggersModal').style.display = 'none';
  configuringTriggersForTaskId = null;
}

function saveTriggers() {
  const task = findTaskById(configuringTriggersForTaskId);
  if (!task) return;

  const checkboxes = document.querySelectorAll('.trigger-checkbox');
  task.triggers = [];
  checkboxes.forEach(cb => {
    if (cb.checked) {
      task.triggers.push(parseFloat(cb.dataset.bgTaskId));
    }
  });

  saveTasks();
  renderTasks();
  closeTriggersModal();
}

function addNewBackgroundTaskFromTriggerModal() {
  const input = document.getElementById('newBgTaskInput');
  const text = input ? input.value.trim() : '';
  if (!text) {
    alert('Please enter a background task description.');
    return;
  }

  const highPriority = document.getElementById('bgTaskHighPriority')?.checked || false;

  addBackgroundTask(text, {
    priority: highPriority ? 'high' : 'normal'
  });
  saveTasks();

  // Re-open the modal to show the new task
  configureTriggers(configuringTriggersForTaskId);
}

// Restart expiration timers on app load for active background tasks
function restartExpirationTimers() {
  const activeBgTasks = tasks.filter(t => t.mode === 'background' && t.activated && !t.completed);
  activeBgTasks.forEach(bgTask => {
    if (bgTask.backgroundOptions?.expiresAfterMinutes && bgTask.activatedAt) {
      startExpirationTimer(bgTask);
    }
  });
}

// Find the parent task of a given child ID
function findParentTask(childId, taskList = tasks) {
  for (const task of taskList) {
    if (task.children) {
      for (const child of task.children) {
        if (child.id === childId) {
          return task;
        }
      }
      const found = findParentTask(childId, task.children);
      if (found) return found;
    }
  }
  return null;
}

function toggleTask(taskId) {
  const task = findTaskById(taskId);
  if (!task) return;

  pushUndoState('toggle task');
  task.completed = !task.completed;

  // If parent is being completed, complete all children
  if (task.completed && task.children && task.children.length > 0) {
    completeAllChildren(task);
  }

  // If child is being toggled, check if parent should auto-complete
  if (task.completed) {
    checkParentCompletion(taskId);
    // Fire triggers for background tasks
    activateTriggeredTasks(task);
    // Also check if parent was auto-completed and fire its triggers
    const parent = findParentTask(taskId);
    if (parent && parent.completed) {
      activateTriggeredTasks(parent);
    }
  } else {
    // If child is unchecked, uncheck parent
    uncheckParent(taskId);
    // Deactivate triggered background tasks that haven't been independently completed
    deactivateTriggeredTasks(task);
  }

  renderTasks();
  updateProgress();
  saveTasks();
}

function deleteTask(taskId) {
  const taskToDelete = findTaskById(taskId);
  if (!taskToDelete) return;

  if (!confirm(`Delete "${taskToDelete.text}"? You can undo this with Ctrl+Shift+Z.`)) {
    return;
  }

  pushUndoState('delete task');

  // Clean up trigger references pointing to this task
  function removeTriggerReferences(taskList) {
    taskList.forEach(task => {
      if (task.triggers) {
        task.triggers = task.triggers.filter(id => id !== taskId);
      }
      if (task.children) removeTriggerReferences(task.children);
    });
  }
  removeTriggerReferences(tasks);

  // Clear any expiration timer
  if (expirationTimers[taskId]) {
    clearTimeout(expirationTimers[taskId]);
    delete expirationTimers[taskId];
  }

  function removeTaskRecursively(taskList) {
    for (let i = 0; i < taskList.length; i++) {
      if (taskList[i].id === taskId) {
        taskList.splice(i, 1);
        return true;
      }
      if (taskList[i].children && removeTaskRecursively(taskList[i].children)) {
        return true;
      }
    }
    return false;
  }

  if (removeTaskRecursively(tasks)) {
    renderTasks();
    updateProgress();
    saveTasks();
  }
}

function clearAllTasks() {
  const totalTasks = countAllTasks();
  if (totalTasks === 0) return;

  if (confirm(`Clear all ${totalTasks} task${totalTasks === 1 ? '' : 's'}? You can undo this with Ctrl+Shift+Z.`)) {
    pushUndoState('clear all tasks');
    tasks.splice(0, tasks.length);
    renderTasks();
    updateProgress();
    saveTasks();
  }
}


  async function loadTasks() {
    try {
      const data = await ipcRenderer.invoke('load-tasks');
      tasks.splice(0, tasks.length, ...(data.tasks || []));
      state.currentTemplate = data.currentTemplate;
      state.snapshots = Array.isArray(data.snapshots)
        ? data.snapshots.slice(-MAX_AUTO_SAVE_SNAPSHOTS)
        : [];
    } catch (error) {
      console.error('Failed to load tasks:', error);
    }
  }

  async function saveTasks() {
    try {
      recordAutoSaveSnapshot();
      await ipcRenderer.invoke('save-tasks', {
        tasks: tasks,
        currentTemplate: state.currentTemplate,
        snapshots: state.snapshots
      });
    } catch (error) {
      console.error('Failed to save tasks:', error);
    }
  }

  return {
    setupInteractiveModeListener,
    completeNextTask,
    findNextLeafTask,
    addTask,
    migrateTaskStructure,
    findTaskById,
    completeAllChildren,
    checkParentCompletion,
    uncheckParent,
    activateTriggeredTasks,
    deactivateTriggeredTasks,
    showBackgroundTaskNotification,
    startExpirationTimer,
    dismissBackgroundTask,
    addBackgroundTask,
    configureTriggers,
    closeTriggersModal,
    saveTriggers,
    addNewBackgroundTaskFromTriggerModal,
    restartExpirationTimers,
    findParentTask,
    toggleTask,
    undoLastAction,
    deleteTask,
    clearAllTasks,
    loadTasks,
    saveTasks
  };
}
