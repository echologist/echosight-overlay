export function createUiRenderer(ctx) {
  const { ipcRenderer, state } = ctx;
  const { tasks, settings } = state;

  function initializeDragAndDrop() {
    ctx.managers.dragDrop.initializeDragAndDrop();
  }

  function findTaskById(id) {
    return ctx.managers.task.findTaskById(id);
  }

  function saveTasks() {
    return ctx.managers.task.saveTasks();
  }

  function updateProgress() {
    ctx.managers.progress.updateProgress();
  }

  function configureTriggers(taskId) {
    ctx.managers.task.configureTriggers(taskId);
  }

function updateInteractiveVisuals(interactive) {
  const container = document.getElementById('overlayContainer');
  const hint = document.getElementById('shortcut-hint');
  const toggleBtn = document.getElementById('interactiveToggle');

  if (interactive) {
    container.classList.add('interactive');
    container.classList.remove('click-through');

    // Add glassmorphism class if using glass background
    if (settings.backgroundColor === 'glass') {
      container.classList.add('glass-mode');
    } else {
      container.classList.remove('glass-mode');
    }

    if (hint) {
      hint.textContent = '(Interactive Mode - Ctrl+Shift+I to exit)';
      hint.style.color = '#ffffff';
    }
    if (toggleBtn) {
      toggleBtn.textContent = 'Interactive Mode';
      toggleBtn.style.background = '#32cd32';
    }
  } else {
    container.classList.remove('interactive');
    container.classList.remove('glass-mode');
    container.classList.add('click-through');
    // No need to update hint since header is hidden in click-through mode
    if (toggleBtn) {
      toggleBtn.textContent = 'Click-Through Mode';
      toggleBtn.style.background = '#666';
    }
  }
  
  // Theme now handles transparency properly, no override needed
}

// Enhanced renderTasks with hierarchical structure and background task support
function renderTasks() {
  const taskList = document.getElementById('taskList');
  const backgroundTaskList = document.getElementById('backgroundTaskList');
  const backgroundSection = document.getElementById('backgroundTasksSection');

  taskList.innerHTML = '';
  backgroundTaskList.innerHTML = '';

  const mainTasks = tasks.filter(t => t.mode !== 'background');
  const activeBackgroundTasks = tasks.filter(t => t.mode === 'background' && t.activated);

  function renderTaskLevel(taskArray, level = 0, targetList = taskList, isBackground = false) {
    taskArray.forEach(task => {
      const li = document.createElement('li');
      li.className = 'task-item';
      li.dataset.taskId = task.id;
      li.dataset.level = level;
      li.dataset.mode = task.mode || 'main';
      li.draggable = true;

      if (isBackground && task.backgroundOptions?.priority === 'high') {
        li.classList.add('high-priority');
      }

      const isParent = task.children && task.children.length > 0;
      const indentPadding = level * 20;
      const hasTriggers = task.triggers && task.triggers.length > 0;

      li.innerHTML = `
        <div style="padding-left: ${indentPadding}px; display: flex; align-items: center; width: 100%;">
          <span class="drag-handle" style="
            cursor: grab;
            color: #666;
            font-size: 14px;
            user-select: none;
            padding: 2px 4px 2px 0;
            opacity: 0.5;
            transition: opacity 0.2s ease;
            display: inline-block;
          " title="Drag to reorder">⋮⋮</span>

          ${isParent ?
            `<span class="parent-icon" style="
              color: #d4af37;
              font-size: 12px;
              margin-right: 4px;
              user-select: none;
            ">📁</span>` :
            `<span style="width: 16px; margin-right: 4px;"></span>`
          }

          <input type="checkbox" class="task-checkbox" ${task.completed ? 'checked' : ''}
                 onchange="toggleTask(${task.id})">
          <span class="task-text ${task.completed ? 'completed' : ''}"
                style="${isParent ? 'font-weight: bold;' : ''}"
                oncontextmenu="showContextMenu(event, ${task.id}, ${isParent})">${escapeHtml(task.text)}</span>

          <span class="task-badges">
            ${hasTriggers ?
              `<span class="trigger-indicator" onclick="configureTriggers(${task.id})" title="Triggers ${task.triggers.length} background task(s)">&#9889;${task.triggers.length}</span>` :
              ''
            }

            ${isParent ?
              `<span class="child-count">(${task.children.filter(c => c.completed).length}/${task.children.length})</span>` :
              ''
            }
          </span>

          <span class="right-click-hint">Right-click for options</span>

          <button class="task-delete" onclick="deleteTask(${task.id})">×</button>
        </div>
      `;

      targetList.appendChild(li);

      // Render children if they exist
      if (task.children && task.children.length > 0) {
        renderTaskLevel(task.children, level + 1, targetList, isBackground);
      }
    });
  }

  renderTaskLevel(mainTasks, 0, taskList, false);
  renderTaskLevel(activeBackgroundTasks, 0, backgroundTaskList, true);

  // Show/hide background section
  if (backgroundSection) {
    backgroundSection.style.display = activeBackgroundTasks.length > 0 ? 'block' : 'none';
    const bgCount = document.getElementById('backgroundCount');
    if (bgCount) {
      bgCount.textContent = activeBackgroundTasks.length > 0
        ? `(${activeBackgroundTasks.filter(t => t.completed).length}/${activeBackgroundTasks.length})`
        : '';
    }
  }

  initializeDragAndDrop();
}


// Context menu for adding subtasks
let contextMenu = null;

function showContextMenu(event, taskId, isParent) {
  event.preventDefault();
  
  // Remove existing context menu
  if (contextMenu) {
    contextMenu.remove();
  }
  
  // Only show context menu for tasks (any task can become a parent)
  contextMenu = document.createElement('div');
  contextMenu.className = 'context-menu';
  contextMenu.style.cssText = `
    position: fixed;
    background: #222;
    border: 1px solid #555;
    border-radius: 4px;
    padding: 4px 0;
    z-index: 1000;
    font-size: 12px;
    box-shadow: 0 2px 10px rgba(0,0,0,0.5);
  `;
  
  const menuItem = document.createElement('div');
  menuItem.textContent = 'Add sub-task';
  menuItem.style.cssText = `
    padding: 6px 12px;
    cursor: pointer;
    color: white;
    white-space: nowrap;
  `;
  
  menuItem.addEventListener('mouseover', () => {
    menuItem.style.background = '#444';
  });
  
  menuItem.addEventListener('mouseout', () => {
    menuItem.style.background = 'transparent';
  });
  
  menuItem.addEventListener('click', () => {
    addSubTask(taskId);
    contextMenu.remove();
    contextMenu = null;
  });

  contextMenu.appendChild(menuItem);

  // Add "Configure Triggers" option
  const triggerItem = document.createElement('div');
  triggerItem.textContent = 'Configure Triggers';
  triggerItem.style.cssText = `
    padding: 6px 12px;
    cursor: pointer;
    color: white;
    white-space: nowrap;
  `;

  triggerItem.addEventListener('mouseover', () => {
    triggerItem.style.background = '#444';
  });

  triggerItem.addEventListener('mouseout', () => {
    triggerItem.style.background = 'transparent';
  });

  triggerItem.addEventListener('click', () => {
    contextMenu.remove();
    contextMenu = null;
    configureTriggers(taskId);
  });

  // Override the default :before pseudo-element for this item
  triggerItem.style.display = 'flex';
  triggerItem.style.alignItems = 'center';
  triggerItem.style.fontWeight = '500';

  contextMenu.appendChild(triggerItem);
  document.body.appendChild(contextMenu);
  
  // Position the context menu
  contextMenu.style.left = event.pageX + 'px';
  contextMenu.style.top = event.pageY + 'px';
  
  // Remove context menu when clicking elsewhere
  setTimeout(() => {
    document.addEventListener('click', function removeMenu() {
      if (contextMenu) {
        contextMenu.remove();
        contextMenu = null;
      }
      document.removeEventListener('click', removeMenu);
    });
  }, 10);
}

// Store the parent ID for the modal
let currentParentId = null;

function addSubTask(parentId) {
  console.log('addSubTask called with parentId:', parentId);
  currentParentId = parentId;
  
  // Show the modal
  document.getElementById('addSubTaskModal').style.display = 'flex';
  
  // Ensure the window has focus, then focus the input
  setTimeout(() => {
    // Try to focus the window first (Electron specific bs)
    if (window.focus) {
      window.focus();
    }
    
    // Then focus the input
    const input = document.getElementById('subTaskInput');
    if (input) {
      input.focus();
      input.select();
    }
  }, 150);
}

function closeSubTaskModal() {
  document.getElementById('addSubTaskModal').style.display = 'none';
  document.getElementById('subTaskInput').value = '';
  currentParentId = null;
  
  // Return focus to the main window
  setTimeout(() => {
    if (window.focus) {
      window.focus();
    }
  }, 50);
}

function saveSubTask() {
  const taskText = document.getElementById('subTaskInput').value.trim();
  console.log('User entered:', taskText);
  
  if (taskText && currentParentId) {
    const parentTask = findTaskById(currentParentId);
    console.log('Found parent task:', parentTask);
    
    if (parentTask) {
      const subTask = {
        id: Date.now(),
        text: taskText,
        completed: false,
        createdAt: new Date().toISOString(),
        children: [],
        mode: 'main',
        triggers: [],
        activated: true,
        activatedAt: null,
        backgroundOptions: null
      };
      
      parentTask.children.push(subTask);
      console.log('Added subtask, parent now has:', parentTask.children.length, 'children');
      renderTasks();
      updateProgress();
      saveTasks();
      closeSubTaskModal();
    }
  } else if (!taskText) {
    alert('Please enter a task name!');
  }
}


function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

function toggleOverlay() {
  try {
    console.log('toggleOverlay called');
    ipcRenderer.send('toggle-overlay');
  } catch (error) {
    console.error('Error toggling overlay:', error);
  }
}

function minimizeOverlay() {
  try {
    console.log('minimizeOverlay called');
    ipcRenderer.send('minimize-overlay');
  } catch (error) {
    console.error('Error minimizing overlay:', error);
  }
}

function closeOverlay() {
  try {
    console.log('closeOverlay called - quitting application');
    if (confirm('Close Echosight Overlay completely?')) {
      ipcRenderer.send('quit-application');
    }
  } catch (error) {
    console.error('Error closing overlay:', error);
  }
}

function toggleInteractiveMode() {
  try {
    console.log('toggleInteractiveMode called');
    ipcRenderer.send('toggle-interactive-mode');
  } catch (error) {
    console.error('Error toggling interactive mode:', error);
  }
}


  return {
    updateInteractiveVisuals,
    renderTasks,
    showContextMenu,
    addSubTask,
    closeSubTaskModal,
    saveSubTask,
    escapeHtml,
    toggleOverlay,
    minimizeOverlay,
    closeOverlay,
    toggleInteractiveMode
  };
}
