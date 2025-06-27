// ES Module imports 4 Vite
const { ipcRenderer } = window.require('electron');

let tasks = [];
let templates = [];
let themes = [];
let currentTemplate = null;
let isInteractiveMode = true;
let settings = {
  transparency: 25,
  theme: 'dark',
  hotkeys: {
    toggleVisibility: 'Ctrl+Shift+T',
    toggleInteractive: 'Ctrl+Shift+I',
    completeNextTask: 'Ctrl+Shift+N'
  }
}

function resetHotkeys() {
  if (confirm('Reset hotkeys to default values (Ctrl+Shift+T, Ctrl+Shift+I, and Ctrl+Shift+N)?')) {
    settings.hotkeys.toggleVisibility = 'Ctrl+Shift+T';
    settings.hotkeys.toggleInteractive = 'Ctrl+Shift+I';
    settings.hotkeys.completeNextTask = 'Ctrl+Shift+N';

    // Update the input fields
    document.getElementById('toggleVisibilityHotkey').value = 'Ctrl+Shift+T';
    document.getElementById('toggleInteractiveHotkey').value = 'Ctrl+Shift+I';
    document.getElementById('completeNextTaskHotkey').value = 'Ctrl+Shift+N';

    alert('Hotkeys reset to defaults. Click "Save Settings" to apply.');
  }
}

// Settings Management
function showSettingsModal() {
  document.getElementById('settingsModal').style.display = 'flex';

  updateThemeSelector();
  
  document.getElementById('transparencySlider').value = settings.transparency;
  document.getElementById('transparencyValue').textContent = settings.transparency + '% visible';
  document.getElementById('themeSelect').value = settings.theme;
  document.getElementById('toggleVisibilityHotkey').value = settings.hotkeys.toggleVisibility;
  document.getElementById('toggleInteractiveHotkey').value = settings.hotkeys.toggleInteractive;
  document.getElementById('completeNextTaskHotkey').value = settings.hotkeys.completeNextTask;
}

function updateThemeSelector() {
  const select = document.getElementById('themeSelect');
  select.innerHTML = '';

  themes.forEach(theme => {
    const option = document.createElement('option');
    option.value = theme.id;
    option.textContent = theme.name;
    if (theme.description) {
      option.title = theme.description;
    }
    select.appendChild(option);
  });
}

function closeSettingsModal() {
  // Stop any active hotkey recording safely
  try {
    stopRecording();
  } catch (error) {
    console.log('Error stopping recording:', error);
  }
  document.getElementById('settingsModal').style.display = 'none';
}

function updateTransparency(value) {
  document.getElementById('transparencyValue').textContent = value + '% visible';

  // Apply transparency preview immediately
  settings.transparency = parseInt(value);
  applyTransparencySettings();
}

function updateTheme(themeId) {
  console.log('Theme changed to:', themeId);
  settings.theme = themeId;
  applyTheme();

  if (isInteractiveMode) {
    updateInteractiveVisuals(true);
  }
}

async function applyTheme() {
  try {
    const theme = await getCurrentTheme();
    if (!theme) {
      console.error('No theme found for ID:', settings.theme);
      return;
    }

    console.log('Applying theme:', theme.name, 'with settings:', settings);
    
    removeExistingStyles();
    
    const style = document.createElement('style');
    style.id = 'theme-style';
    
    let css = generateThemeCSS(theme);
    console.log('Generated CSS:', css);
    style.textContent = css;
    document.head.appendChild(style);
    
    applyFonts(theme);
    
  } catch (error) {
    console.error('Failed to apply theme:', error);
  }
}

function removeExistingStyles() {
  const existingStyles = ['theme-style', 'background-style', 'transparency-style', 'font-style'];
  existingStyles.forEach(id => {
    const element = document.getElementById(id);
    if (element) element.remove();
  });
}

function generateThemeCSS(theme) {
  const transparency = settings.transparency / 100;
  
  const cssVars = generateCSSVariables(theme, transparency);
  const interactiveStyles = generateInteractiveStyles(theme);
  const clickThroughStyles = generateClickThroughStyles(theme);
  const commonStyles = generateCommonStyles(theme);
  
  return `
    :root {
      ${cssVars}
    }
    
    ${commonStyles}
    ${interactiveStyles}
    ${clickThroughStyles}
  `;
}

function generateCSSVariables(theme, transparency) {
  const vars = [];
  
  vars.push(`--user-transparency: ${transparency}`);
  
  Object.entries(theme.colors).forEach(([category, values]) => {
    if (typeof values === 'object') {
      Object.entries(values).forEach(([key, value]) => {
        const shortCategory = category === 'background' ? 'bg' : 
                             category === 'border' ? 'border' :
                             category === 'text' ? 'text' : category;
        vars.push(`--${shortCategory}-${key}: ${value}`);
      });
    } else {
      vars.push(`--${category}: ${values}`);
    }
  });
  
  if (theme.effects) {
    Object.entries(theme.effects).forEach(([key, value]) => {
      vars.push(`--${key.replace(/([A-Z])/g, '-$1').toLowerCase()}: ${value}`);
    });
  }
  
  return vars.join(';\n      ');
}

function generateInteractiveStyles(theme) {
  const interactive = theme.styles.interactive;
  if (!interactive) return '';
  
  return `
    .overlay-container.interactive {
      ${objectToCSS(interactive.container)}
    }
    
    .overlay-container.interactive .header {
      ${objectToCSS(interactive.header)}
    }
    
    .overlay-container.interactive .template-section {
      ${objectToCSS(interactive.templateSection)}
    }
    
    .overlay-container.interactive .tasks-section {
      ${objectToCSS(interactive.tasksSection)}
    }
  `;
}

function generateClickThroughStyles(theme) {
  const clickThrough = theme.styles.clickThrough;
  if (!clickThrough) return '';
  
  return `
    .overlay-container.click-through {
      ${objectToCSS(clickThrough.container)}
    }
    
    .overlay-container.click-through .tasks-section {
      ${objectToCSS(clickThrough.tasksSection)}
    }
  `;
}

function generateCommonStyles(theme) {
  return `
    .overlay-container .task-text {
      color: var(--text-primary) !important;
      font-weight: var(--font-weight-normal, normal) !important;
      ${theme.effects?.textShadow ? `text-shadow: var(--text-shadow) !important;` : ''}
    }
    
    .overlay-container .task-text.completed {
      color: var(--text-muted) !important;
      ${theme.effects?.textShadow ? `text-shadow: var(--text-shadow) !important;` : ''}
    }
    
    .overlay-container .progress-text {
      color: var(--text-secondary) !important;
      font-weight: var(--font-weight-bold, bold) !important;
      ${theme.effects?.textShadow ? `text-shadow: var(--text-shadow) !important;` : ''}
    }
    
    .overlay-container .progress-bar {
      ${theme.effects?.boxShadow ? `box-shadow: var(--box-shadow) !important;` : ''}
    }
    
    .overlay-container .task-checkbox {
      ${theme.effects?.dropShadow ? `filter: var(--drop-shadow) !important;` : ''}
    }
  `;
}

function objectToCSS(obj) {
  if (!obj) return '';
  
  return Object.entries(obj).map(([key, value]) => {
    const cssKey = key.replace(/([A-Z])/g, '-$1').toLowerCase();
    return `${cssKey}: ${value} !important`;
  }).join(';\n      ');
}

async function getCurrentTheme() {
  const themeId = settings.theme || 'dark';
  return await ipcRenderer.invoke('get-theme', themeId);
}

function applyFonts(theme) {
  if (!theme.fonts) return;
  
  const style = document.createElement('style');
  style.id = 'font-style';
  
  let fontCSS = '';
  
  if (theme.fonts.primary) {
    const font = theme.fonts.primary;
    
    fontCSS += `
      body, .overlay-container {
        font-family: ${font.family} !important;
      }
      
      .header h2 {
        font-size: ${font.sizes.header} !important;
      }
      
      .task-text {
        font-size: ${font.sizes.task} !important;
      }
      
      .progress-text {
        font-size: ${font.sizes.progress} !important;
      }
      
      button, select, input {
        font-size: ${font.sizes.ui} !important;
        font-family: ${font.family} !important;
      }
    `;
    
    if (font.weights) {
      fontCSS += `
        :root {
          --font-weight-normal: ${font.weights.normal};
          --font-weight-bold: ${font.weights.bold};
        }
      `;
    }
  }
  
  style.textContent = fontCSS;
  document.head.appendChild(style);
}

function applyTransparencySettings() {
  applyTheme();
}

async function saveSettings() {
  try {
    console.log('Saving settings...');

    // Stop any active recording first
    if (typeof recordingHotkey !== 'undefined' && recordingHotkey) {
      stopRecording();
    }

    // Get all settings values
    settings.transparency = parseInt(document.getElementById('transparencySlider').value);
    settings.theme = document.getElementById('themeSelect').value;

    // Make sure hotkeys object exists
    if (!settings.hotkeys) {
      settings.hotkeys = {
        toggleVisibility: 'Ctrl+Shift+T',
        toggleInteractive: 'Ctrl+Shift+I',
        completeNextTask: 'Ctrl+Shift+N'
      };
    }

    // Get current hotkey values from input fields (in case recording didn't update them)
    const visibilityInput = document.getElementById('toggleVisibilityHotkey');
    const interactiveInput = document.getElementById('toggleInteractiveHotkey');
    const completeTaskInput = document.getElementById('completeNextTaskHotkey');

    if (visibilityInput.value.trim()) {
      settings.hotkeys.toggleVisibility = visibilityInput.value.trim();
    }
    if (interactiveInput.value.trim()) {
      settings.hotkeys.toggleInteractive = interactiveInput.value.trim();
    }
    if (completeTaskInput.value.trim()) {
      settings.hotkeys.completeNextTask = completeTaskInput.value.trim();
    }

    console.log('Current settings:', settings);

    // Save to storage
    const saveResult = await ipcRenderer.invoke('save-settings', settings);
    console.log('Save result:', saveResult);

    if (!saveResult.success) {
      throw new Error(saveResult.error || 'Unknown save error');
    }

    // Send hotkey changes to main process
    console.log('Updating hotkeys:', settings.hotkeys);
    ipcRenderer.send('update-hotkeys', settings.hotkeys);

    closeSettingsModal();
    alert('Settings saved successfully! Hotkey changes will take effect after restarting the overlay.');

  } catch (error) {
    console.error('Error saving settings:', error);
    alert(`Error saving settings: ${error.message}. Please try again.`);
  }
}

async function loadSettings() {
  try {
    console.log('Loading settings...');
    const loadedSettings = await ipcRenderer.invoke('load-settings');
    console.log('Loaded settings:', loadedSettings);

    if (loadedSettings) {
      settings = {
        ...settings,
        ...loadedSettings,
        hotkeys: {
          ...settings.hotkeys,
          ...loadedSettings.hotkeys
        }
      };
      
      if (loadedSettings.backgroundColor && !loadedSettings.theme) {
        console.log('Migrating backgroundColor to theme:', loadedSettings.backgroundColor);
        settings.theme = loadedSettings.backgroundColor;
      }
    }
    console.log('Final settings:', settings);
  } catch (error) {
    console.error('Failed to load settings:', error);
  }
}

function resetPosition() {
  ipcRenderer.send('reset-window-position');
  alert('Window position reset! The overlay will move to the top-right corner.');
}

// Hotkey recording system
function recordHotkey(hotkeyType) {
  window.recordingHotkey = hotkeyType;
  const input = document.getElementById(hotkeyType + 'Hotkey');
  let button;

  if (hotkeyType === 'toggleVisibility') {
    button = document.getElementById('recordBtn1');
  } else if (hotkeyType === 'toggleInteractive') {
    button = document.getElementById('recordBtn2');
  } else if (hotkeyType === 'completeNextTask') {
    button = document.getElementById('recordBtn3');
  }

  input.value = 'Press keys now...';
  input.style.background = '#444';
  button.textContent = 'Recording...';
  button.style.background = '#dc143c';
  button.disabled = true;

  // Focus on the input to capture keys
  input.focus();
}

function stopRecording() {
  if (typeof window.recordingHotkey !== 'undefined' && window.recordingHotkey) {
    let button;
    if (window.recordingHotkey === 'toggleVisibility') {
      button = document.getElementById('recordBtn1');
    } else if (window.recordingHotkey === 'toggleInteractive') {
      button = document.getElementById('recordBtn2');
    } else if (window.recordingHotkey === 'completeNextTask') {
      button = document.getElementById('recordBtn3');
    }

    const input = document.getElementById(window.recordingHotkey + 'Hotkey');

    if (input) {
      input.style.background = '#333';
    }
    if (button) {
      button.textContent = 'Record';
      button.style.background = '#666';
      button.disabled = false;
    }
    window.recordingHotkey = null;
  }
}

function handleHotkeyRecording(event) {
  if (!window.recordingHotkey) return;

  event.preventDefault();
  event.stopPropagation();

  const keys = [];

  // Collect modifier keys
  if (event.ctrlKey || event.metaKey) keys.push('Ctrl');
  if (event.altKey) keys.push('Alt');
  if (event.shiftKey) keys.push('Shift');

  // Add the main key (if it's not a modifier)
  if (!['Control', 'Alt', 'Shift', 'Meta'].includes(event.key)) {
    let mainKey = event.key;

    // Convert some keys to more readable format
    switch (event.key) {
      case ' ': mainKey = 'Space'; break;
      case 'ArrowUp': mainKey = 'Up'; break;
      case 'ArrowDown': mainKey = 'Down'; break;
      case 'ArrowLeft': mainKey = 'Left'; break;
      case 'ArrowRight': mainKey = 'Right'; break;
      default:
        if (mainKey.length === 1) {
          mainKey = mainKey.toUpperCase();
        }
    }

    keys.push(mainKey);

    // Validate the key combination to prevent typing conflicts
    const hasRequiredModifier = keys.includes('Ctrl') || keys.includes('Alt') ||
      ['F1', 'F2', 'F3', 'F4', 'F5', 'F6', 'F7', 'F8', 'F9', 'F10', 'F11', 'F12'].includes(mainKey);

    // Warn about problematic combinations
    if (keys.length === 2 && keys.includes('Shift') && mainKey.length === 1) {
      alert(`Warning: "${keys.join('+')}" may interfere with typing. Consider using Ctrl+${mainKey} or Alt+${mainKey} instead.`);
    }

    // Only save if we have a complete key combination
    if (hasRequiredModifier) {
      const hotkeyString = keys.join('+');
      const input = document.getElementById(window.recordingHotkey + 'Hotkey');
      input.value = hotkeyString;

      // Update settings
      settings.hotkeys[window.recordingHotkey] = hotkeyString;

      // Stop recording after a short delay
      setTimeout(() => {
        stopRecording();
      }, 200);
    } else if (keys.length >= 2) {
      // Show warning for insufficient modifiers
      alert('Please use Ctrl+Key, Alt+Key, or Function keys to avoid conflicts with normal typing.');
    }
  }
}

function previewBackground() {
  // Temporarily switch to read-only mode to preview background
  const wasInteractive = isInteractiveMode;
  if (wasInteractive) {
    // Temporarily switch to click-through mode
    updateInteractiveVisuals(false);
    setTimeout(() => {
      // Switch back after preview
      updateInteractiveVisuals(true);
    }, 3000);
    alert('Switched to read-only mode for 3 seconds to preview background style.');
  } else {
    alert('Background style is already visible in read-only mode.');
  }
}

// Initialize app
async function initializeApp() {
  try {
    console.log('Initializing app...');
    await loadTasks();
    migrateTasksToHierarchical();
    await loadTemplates();
    await loadThemes();
    await loadSettings();
    updateTemplateSelect();
    renderTasks();
    updateProgress();
    setupInteractiveModeListener();
    await applyTheme();
    console.log('App initialized successfully');
  } catch (error) {
    console.error('Error initializing app:', error);
    alert('Error initializing app. Check console for details.');
  }
}

async function loadThemes() {
  try {
    themes = await ipcRenderer.invoke('load-themes');
    console.log('Themes loaded:', themes.length);
  } catch (error) {
    console.error('Failed to load themes:', error);
    themes = [];
  }
}

async function openThemesFolder() {
  try {
    const result = await ipcRenderer.invoke('open-themes-folder');
    if (result.success) {
      console.log('Themes folder opened');
    } else {
      alert('Failed to open themes folder: ' + result.error);
    }
  } catch (error) {
    console.error('Error opening themes folder:', error);
    alert('Error opening themes folder. Check console for details.');
  }
}

async function reloadThemes() {
  try {
    themes = await ipcRenderer.invoke('reload-themes');
    updateThemeSelector();
    const currentTheme = document.getElementById('themeSelect').value;
    if (currentTheme) {
      settings.theme = currentTheme;
      await applyTheme();
    }
    alert(`Themes reloaded! Found ${themes.length} themes.`);
  } catch (error) {
    console.error('Error reloading themes:', error);
    alert('Error reloading themes. Check console for details.');
  }
}

async function showThemesPath() {
  try {
    const themesPath = await ipcRenderer.invoke('get-themes-path');
    console.log('Themes folder location:', themesPath);
    
    const notification = document.createElement('div');
    notification.style.cssText = `
      position: fixed;
      top: 20px;
      right: 20px;
      background: rgba(0, 0, 0, 0.9);
      color: #d4af37;
      padding: 15px 20px;
      border: 2px solid #d4af37;
      border-radius: 8px;
      font-size: 12px;
      z-index: 1001;
      max-width: 400px;
      line-height: 1.4;
    `;
    notification.innerHTML = `
      <strong>Custom Themes Folder:</strong><br>
      <code style="background: rgba(255,255,255,0.1); padding: 2px 4px; border-radius: 3px; display: block; margin: 8px 0; word-break: break-all;">${themesPath}</code>
      <small>Add .json theme files here and click "Reload" in settings</small>
    `;
    
    document.body.appendChild(notification);
    
    setTimeout(() => {
      if (notification.parentNode) {
        notification.remove();
      }
    }, 8000);
    
  } catch (error) {
    console.error('Error getting themes path:', error);
  }
}

// Handle interactive mode changes from main process
function setupInteractiveModeListener() {
  ipcRenderer.on('interactive-mode-changed', (event, interactive) => {
    console.log('Interactive mode changed:', interactive);
    isInteractiveMode = interactive;
    updateInteractiveVisuals(interactive);
  });

  // Handle complete next task from main process
  ipcRenderer.on('complete-next-task', () => {
    console.log('Complete next task triggered');
    completeNextTask();
  });
}

// Complete the next uncompleted task
// Complete the next uncompleted task - only works on leaf tasks (children)
function completeNextTask() {
  try {
    const nextTask = findNextLeafTask();
    if (nextTask) {
      nextTask.completed = true;
      
      // Check if parent should auto-complete
      checkParentCompletion(nextTask.id);
      
      renderTasks();
      updateProgress();
      saveTasks();
      console.log('Completed task:', nextTask.text);

      // Optional: Show a brief visual feedback
      if (isInteractiveMode) {
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

// Find next uncompleted leaf task (skip parents)
function findNextLeafTask(taskList = tasks) {
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
        children: []
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

// Migration function to add children array to existing tasks
function migrateTasksToHierarchical() {
  let migrated = false;
  tasks = tasks.map(task => {
    if (!task.children) {
      task.children = [];
      migrated = true;
    }
    return task;
  });
  if (migrated) {
    console.log('Tasks migrated to hierarchical structure');
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

  task.completed = !task.completed;

  // If parent is being completed, complete all children
  if (task.completed && task.children && task.children.length > 0) {
    completeAllChildren(task);
  }

  // If child is being toggled, check if parent should auto-complete
  if (task.completed) {
    checkParentCompletion(taskId);
  } else {
    // If child is unchecked, uncheck parent
    uncheckParent(taskId);
  }

  renderTasks();
  updateProgress();
  saveTasks();
}

function deleteTask(taskId) {
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
  if (confirm('Are you sure you want to clear all tasks?')) {
    tasks = [];
    renderTasks();
    updateProgress();
    saveTasks();
  }
}

// Template management
function showSaveTemplateModal() {
  if (tasks.length === 0) {
    alert('No tasks to save as template!');
    return;
  }
  document.getElementById('saveTemplateModal').style.display = 'flex';
  document.getElementById('templateNameInput').focus();
}

function closeSaveTemplateModal() {
  document.getElementById('saveTemplateModal').style.display = 'none';
  document.getElementById('templateNameInput').value = '';
}

async function saveTemplate() {
  const nameInput = document.getElementById('templateNameInput');
  const name = nameInput.value.trim();

  if (!name) {
    alert('Please enter a template name!');
    return;
  }

  if (tasks.length === 0) {
    alert('No tasks to save!');
    return;
  }

  const template = {
    id: Date.now(),
    name: name,
    tasks: tasks.filter(t => !t.completed).map(t => ({
      text: t.text
    })),
    createdAt: new Date().toISOString()
  };

  // Remove existing template with same name
  templates = templates.filter(t => t.name !== name);
  templates.push(template);

  await saveTemplates();
  updateTemplateSelect();
  closeSaveTemplateModal();

  alert(`Template "${name}" saved successfully!`);
}

async function loadTemplate() {
  const select = document.getElementById('templateSelect');
  const templateId = parseInt(select.value);

  if (!templateId) return;

  const template = templates.find(t => t.id === templateId);
  if (!template) return;

  if (tasks.some(t => !t.completed) && !confirm('This will replace your current tasks. Continue?')) {
    return;
  }

  // Clear current tasks and load template
  tasks = template.tasks.map((t, index) => ({
    id: Date.now() + index,
    text: t.text,
    completed: false,
    createdAt: new Date().toISOString()
  }));

  currentTemplate = template.name;
  renderTasks();
  updateProgress();
  saveTasks();
}

async function deleteTemplate() {
  const select = document.getElementById('templateSelect');
  const templateId = parseInt(select.value);

  if (!templateId) {
    alert('Please select a template to delete!');
    return;
  }

  const template = templates.find(t => t.id === templateId);
  if (!template) return;

  if (confirm(`Are you sure you want to delete the template "${template.name}"? This cannot be undone.`)) {
    // Remove template from array
    templates = templates.filter(t => t.id !== templateId);

    // Save updated templates
    await saveTemplates();

    // Update dropdown
    updateTemplateSelect();

    alert(`Template "${template.name}" deleted successfully!`);
  }
}

// Export/Import functionality
function exportTemplate() {
  try {
    console.log('Export template called');
    const select = document.getElementById('templateSelect');
    const templateId = parseInt(select.value);

    if (!templateId) {
      alert('Please select a template to export!');
      return;
    }

    const template = templates.find(t => t.id === templateId);
    if (!template) return;

    // Create exportable template data
    const exportData = {
      name: template.name,
      tasks: template.tasks,
      version: "1.0",
      exportedAt: new Date().toISOString(),
      description: `PoE Task Template: ${template.name}`,
      taskCount: template.tasks.length
    };

    // Create and download file
    const dataStr = JSON.stringify(exportData, null, 2);
    const dataBlob = new Blob([dataStr], { type: 'application/json' });

    const link = document.createElement('a');
    link.href = URL.createObjectURL(dataBlob);
    link.download = `poe2-template-${template.name.toLowerCase().replace(/[^a-z0-9]/g, '-')}.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);

    alert(`Template "${template.name}" exported successfully!`);
  } catch (error) {
    console.error('Export error:', error);
    alert('Error exporting template. Please try again.');
  }
}

function showImportModal() {
  document.getElementById('importTemplateModal').style.display = 'flex';
  document.getElementById('importTemplateInput').placeholder = 'Paste template JSON or share code here...';
  document.getElementById('importTemplateInput').focus();
}

function closeImportModal() {
  try {
    document.getElementById('importTemplateModal').style.display = 'none';
    document.getElementById('importTemplateInput').value = '';
  } catch (error) {
    console.error('Error closing import modal:', error);
  }
}

function handleFileImport(event) {
  try {
    const file = event.target.files[0];
    if (!file) return;

    const reader = new FileReader();
    reader.onload = function (e) {
      document.getElementById('importTemplateInput').value = e.target.result;
    };
    reader.readAsText(file);
  } catch (error) {
    console.error('Error handling file import:', error);
    alert('Error reading file. Please try again.');
  }
}

async function importTemplate() {
  const input = document.getElementById('importTemplateInput');
  const inputData = input.value.trim();

  if (!inputData) {
    alert('Please paste template JSON, share code, or select a file!');
    return;
  }

  let templateData;

  try {
    // Try to parse as JSON first
    if (inputData.startsWith('{')) {
      // It's JSON
      templateData = JSON.parse(inputData);

      // Validate JSON template structure
      if (!templateData.name || !templateData.tasks || !Array.isArray(templateData.tasks)) {
        throw new Error('Invalid JSON template format');
      }
    } else {
      // It might be a share code, but we'll just try JSON parsing
      templateData = JSON.parse(inputData);
    }

    // Check if template with same name exists
    const existingTemplate = templates.find(t => t.name === templateData.name);
    if (existingTemplate && !confirm(`Template "${templateData.name}" already exists. Replace it?`)) {
      return;
    }

    // Create new template
    const newTemplate = {
      id: Date.now(),
      name: templateData.name,
      tasks: templateData.tasks.map(task => ({
        text: typeof task === 'string' ? task : task.text
      })),
      createdAt: new Date().toISOString(),
      imported: true
    };

    // Remove existing template with same name
    templates = templates.filter(t => t.name !== templateData.name);
    templates.push(newTemplate);

    await saveTemplates();
    updateTemplateSelect();
    closeImportModal();

    alert(`Template "${templateData.name}" imported successfully! (${newTemplate.tasks.length} tasks)`);

  } catch (error) {
    alert('Invalid template format! Please check the JSON data.');
    console.error('Import error:', error);
  }
}

function updateTemplateSelect() {
  const select = document.getElementById('templateSelect');
  select.innerHTML = '<option value="">Select Template...</option>';

  templates.forEach(template => {
    const option = document.createElement('option');
    option.value = template.id;
    option.textContent = `${template.name} (${template.tasks.length} tasks)`;
    select.appendChild(option);
  });
}

// Community Templates
function getCommunityTemplates() {
  return [
    {
      name: "League Start Essentials",
      description: "Core objectives for starting a new league",
      tasks: [
        "Reach Act 6 for resistance penalty",
        "Complete Normal Labyrinth",
        "Get life/ES nodes on passive tree",
        "Cap resistances (75%+)",
        "Find/buy movement skill gem",
        "Set up basic currency stash tabs",
        "Get weapon with linked sockets",
        "Reach level 68 for endgame content"
      ]
    },
    {
      name: "Endgame Progression",
      description: "Late game goals and pinnacle content",
      tasks: [
        "Complete Atlas progression",
        "Defeat Shaper",
        "Defeat Elder",
        "Complete all Pinnacle bosses",
        "Reach level 90+",
        "Get 6-link main skill",
        "Accumulate 10+ Divine Orbs",
        "Complete Uber Lab trials",
        "Max out important flasks"
      ]
    },
    {
      name: "New Character Setup",
      description: "Essential steps when creating a new character",
      tasks: [
        "Plan passive tree route (PoB)",
        "Identify skill gem progression",
        "Set up loot filter",
        "Transfer currency from main",
        "Get leveling uniques if available",
        "Join guild/find party",
        "Research build guide thoroughly",
        "Prepare gems for later levels"
      ]
    },
    {
      name: "Currency Goals",
      description: "Economic milestones for league",
      tasks: [
        "Save 1 Divine Orb",
        "Save 5 Divine Orbs",
        "Save 20 Divine Orbs",
        "Save 50 Divine Orbs",
        "Get premium stash tab",
        "Set up efficient farming strategy",
        "Learn market prices for key items",
        "Build up crafting materials"
      ]
    },
    {
      name: "HC/SSF Priorities",
      description: "Hardcore and Solo Self-Found specific goals",
      tasks: [
        "Over-cap resistances (85%+)",
        "Get fortify/defensive layers",
        "Level backup gems",
        "Hoard life flasks",
        "Get movement skills early",
        "Plan escape routes",
        "Avoid risky content until geared",
        "Build defensive passive tree first"
      ]
    },
    {
      name: "Crafting Checklist",
      description: "Steps for crafting progression",
      tasks: [
        "Learn basic crafting recipes",
        "Stockpile crafting orbs",
        "Get good base items",
        "Practice on cheaper items first",
        "Research craft of exile",
        "Set up crafting bench",
        "Learn advanced techniques",
        "Plan expensive crafts carefully"
      ]
    }
  ];
}

function loadCommunityTemplates() {
  try {
    console.log('Loading community templates');
    const modal = document.getElementById('communityTemplatesModal');
    const list = document.getElementById('communityTemplatesList');

    const communityTemplates = getCommunityTemplates();

    list.innerHTML = '';
    communityTemplates.forEach((template, index) => {
      const div = document.createElement('div');
      div.style.cssText = `
        border: 1px solid #555;
        border-radius: 6px;
        padding: 12px;
        margin-bottom: 12px;
        background: rgba(0,0,0,0.3);
      `;

      div.innerHTML = `
        <h4 style="color: #d4af37; margin: 0 0 8px 0; font-size: 14px;">${escapeHtml(template.name)}</h4>
        <p style="color: #ccc; margin: 0 0 8px 0; font-size: 12px; font-style: italic;">${escapeHtml(template.description)}</p>
        <p style="color: #aaa; margin: 0 0 10px 0; font-size: 11px;">${template.tasks.length} tasks</p>
        <div style="display: flex; gap: 8px;">
          <button class="modal-btn primary" onclick="importCommunityTemplate(${index})" style="font-size: 12px; padding: 6px 12px;">
            Add to My Templates
          </button>
          <button class="modal-btn secondary" onclick="previewCommunityTemplate(${index})" style="font-size: 12px; padding: 6px 12px;">
            Preview Tasks
          </button>
        </div>
      `;

      list.appendChild(div);
    });

    modal.style.display = 'flex';
  } catch (error) {
    console.error('Error loading community templates:', error);
    alert('Error loading community templates. Please try again.');
  }
}

function closeCommunityModal() {
  try {
    document.getElementById('communityTemplatesModal').style.display = 'none';
  } catch (error) {
    console.error('Error closing community modal:', error);
  }
}

async function importCommunityTemplate(index) {
  try {
    const communityTemplates = getCommunityTemplates();
    const template = communityTemplates[index];

    // Check if template already exists
    const existingTemplate = templates.find(t => t.name === template.name);
    if (existingTemplate && !confirm(`Template "${template.name}" already exists. Replace it?`)) {
      return;
    }

    // Create new template
    const newTemplate = {
      id: Date.now(),
      name: template.name,
      tasks: template.tasks.map(task => ({ text: task })),
      createdAt: new Date().toISOString(),
      community: true
    };

    // Remove existing template with same name
    templates = templates.filter(t => t.name !== template.name);
    templates.push(newTemplate);

    await saveTemplates();
    updateTemplateSelect();

    alert(`Community template "${template.name}" added to your templates!`);
  } catch (error) {
    console.error('Error importing community template:', error);
    alert('Error importing template. Please try again.');
  }
}

function previewCommunityTemplate(index) {
  try {
    const communityTemplates = getCommunityTemplates();
    const template = communityTemplates[index];

    const taskList = template.tasks.map((task, i) => `${i + 1}. ${task}`).join('\n');
    alert(`${template.name} Tasks:\n\n${taskList}`);
  } catch (error) {
    console.error('Error previewing template:', error);
    alert('Error previewing template.');
  }
}

let draggedTaskId = null;
let draggedElement = null;
let placeholder = null;

// Initialize drag and drop functionality
function initializeDragAndDrop() {
  const taskItems = document.querySelectorAll('.task-item');
  
  taskItems.forEach(taskItem => {
    const taskId = parseInt(taskItem.dataset.taskId);
    
    // Make task draggable
    taskItem.draggable = true;
    taskItem.style.cursor = 'grab';
    
    // Add drag event listeners
    taskItem.addEventListener('dragstart', handleDragStart);
    taskItem.addEventListener('dragend', handleDragEnd);
    taskItem.addEventListener('dragover', handleDragOver);
    taskItem.addEventListener('drop', handleDrop);
    taskItem.addEventListener('dragenter', handleDragEnter);
    taskItem.addEventListener('dragleave', handleDragLeave);
  });
}

// Handle drag start
function handleDragStart(e) {
  draggedTaskId = parseInt(e.currentTarget.dataset.taskId);
  draggedElement = e.currentTarget;
  
  // Create visual feedback
  e.currentTarget.style.opacity = '0.5';
  e.currentTarget.style.cursor = 'grabbing';
  
  // Create placeholder element
  placeholder = document.createElement('li');
  placeholder.className = 'drag-placeholder';
  placeholder.style.cssText = `
    height: ${e.currentTarget.offsetHeight}px;
    background: rgba(212, 55, 55, 0.3);
    border: 2px dashed #d4af37;
    border-radius: 4px;
    margin: 2px 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #d4af37;
    font-size: 12px;
    font-style: italic;
    list-style: none;
  `;
  placeholder.textContent = 'Drop here to reorder';
  
  e.dataTransfer.effectAllowed = 'move';
  e.dataTransfer.setData('text/html', e.currentTarget.outerHTML);
}

// Handle drag end
function handleDragEnd(e) {
  // Reset visual feedback
  if (draggedElement) {
    draggedElement.style.opacity = '1';
    draggedElement.style.cursor = 'grab';
  }
  
  // Clean up placeholder
  if (placeholder && placeholder.parentNode) {
    placeholder.parentNode.removeChild(placeholder);
  }
  
  // Reset variables
  draggedTaskId = null;
  draggedElement = null;
  placeholder = null;
}

// Handle drag over
function handleDragOver(e) {
  e.preventDefault();
  e.dataTransfer.dropEffect = 'move';
  
  const targetElement = e.currentTarget;
  if (targetElement === draggedElement) return;
  
  // Determine where to place the placeholder
  const rect = targetElement.getBoundingClientRect();
  const midpoint = rect.top + rect.height / 2;
  const isAbove = e.clientY < midpoint;
  
  // Remove existing placeholder
  if (placeholder && placeholder.parentNode) {
    placeholder.parentNode.removeChild(placeholder);
  }
  
  // Insert placeholder in the correct position
  if (isAbove) {
    targetElement.parentNode.insertBefore(placeholder, targetElement);
  } else {
    targetElement.parentNode.insertBefore(placeholder, targetElement.nextSibling);
  }
}

// Handle drop
function handleDrop(e) {
  e.preventDefault();
  
  if (!draggedTaskId) return;
  
  const targetTaskId = parseInt(e.currentTarget.dataset.taskId);
  if (draggedTaskId === targetTaskId) return;
  
  // Calculate new position
  const targetElement = e.currentTarget;
  const rect = targetElement.getBoundingClientRect();
  const midpoint = rect.top + rect.height / 2;
  const isAbove = e.clientY < midpoint;
  
  // Reorder tasks array
  reorderTasks(draggedTaskId, targetTaskId, isAbove);
}

// Handle drag enter
function handleDragEnter(e) {
  e.preventDefault();
  if (e.currentTarget !== draggedElement) {
    e.currentTarget.style.backgroundColor = 'rgba(212, 175, 55, 0.1)';
  }
}

// Handle drag leave
function handleDragLeave(e) {
  if (e.currentTarget !== draggedElement) {
    e.currentTarget.style.backgroundColor = '';
  }
}

// Reorder tasks in the array
function reorderTasks(draggedId, targetId, insertAbove) {
  // Find the tasks in the array
  const draggedIndex = tasks.findIndex(task => task.id === draggedId);
  const targetIndex = tasks.findIndex(task => task.id === targetId);
  
  if (draggedIndex === -1 || targetIndex === -1) return;
  
  // Remove dragged task from its current position
  const [draggedTask] = tasks.splice(draggedIndex, 1);
  
  // Calculate new insertion index
  let newTargetIndex = tasks.findIndex(task => task.id === targetId);
  let insertIndex = insertAbove ? newTargetIndex : newTargetIndex + 1;
  
  // Insert the dragged task at the new position
  tasks.splice(insertIndex, 0, draggedTask);
  
  // Re-render tasks and save
  renderTasks();
  saveTasks();
}

// Show feedback when task order is updated
function showReorderFeedback() {
  // Create a temporary success message
  const feedback = document.createElement('div');
  feedback.style.cssText = `
    position: fixed;
    top: 20px;
    right: 20px;
    background: rgba(50, 205, 50, 0.9);
    color: white;
    padding: 8px 16px;
    border-radius: 4px;
    font-size: 12px;
    z-index: 1001;
    animation: slideIn 0.3s ease;
  `;
  feedback.textContent = 'Task order updated!';
  
  // Add slide-in animation
  const style = document.createElement('style');
  style.textContent = `
    @keyframes slideIn {
      from { transform: translateX(100%); opacity: 0; }
      to { transform: translateX(0); opacity: 1; }
    }
  `;
  document.head.appendChild(style);
  document.body.appendChild(feedback);
  
  // Remove after 2 seconds
  setTimeout(() => {
    if (feedback.parentNode) {
      feedback.style.animation = 'slideIn 0.3s ease reverse';
      setTimeout(() => feedback.remove(), 300);
    }
    style.remove();
  }, 2000);
}

// Enhanced renderTasks with hierarchical structure
function renderTasks() {
  const taskList = document.getElementById('taskList');
  taskList.innerHTML = '';

  function renderTaskLevel(taskArray, level = 0) {
    taskArray.forEach(task => {
      const li = document.createElement('li');
      li.className = 'task-item';
      li.dataset.taskId = task.id;
      li.dataset.level = level;
      li.draggable = true;
      
      const isParent = task.children && task.children.length > 0;
      const indentPadding = level * 20;
      
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
                 onchange="toggleTask(${task.id})" style>
          <span class="task-text ${task.completed ? 'completed' : ''}" 
                style="${isParent ? 'font-weight: bold;' : ''}"
                oncontextmenu="showContextMenu(event, ${task.id}, ${isParent})">${escapeHtml(task.text)}</span>
          
          ${isParent ? 
            `<span class="child-count" style="
              font-size: 10px;
              color: #aaa;
              margin-left: 8px;
              user-select: none;
            ">(${task.children.filter(c => c.completed).length}/${task.children.length})</span>` : 
            ''
          }
          
          <button class="task-delete" onclick="deleteTask(${task.id})" style="margin-left: auto;">×</button>
        </div>
      `;
      
      taskList.appendChild(li);
      
      // Render children if they exist
      if (task.children && task.children.length > 0) {
        renderTaskLevel(task.children, level + 1);
      }
    });
  }

  renderTaskLevel(tasks);
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
  document.getElementById('subTaskInput').focus();
}

function closeSubTaskModal() {
  document.getElementById('addSubTaskModal').style.display = 'none';
  document.getElementById('subTaskInput').value = '';
  currentParentId = null;
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
        children: []
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

function updateProgress() {
  function countTasks(taskArray) {
    let completed = 0;
    let total = 0;
    
    taskArray.forEach(task => {
      // Count children if they exist, otherwise count the task itself
      if (task.children && task.children.length > 0) {
        const childCounts = countTasks(task.children);
        completed += childCounts.completed;
        total += childCounts.total;
        
        // Also count the parent task
        if (task.completed) completed++;
        total++;
      } else {
        // Leaf task
        if (task.completed) completed++;
        total++;
      }
    });
    
    return { completed, total };
  }

  const { completed, total } = countTasks(tasks);
  const percentage = total > 0 ? (completed / total) * 100 : 0;

  document.getElementById('progressFill').style.width = percentage + '%';
  document.getElementById('progressText').textContent = `${completed} / ${total} tasks completed`;
}

// Data persistence
async function loadTasks() {
  try {
    const data = await ipcRenderer.invoke('load-tasks');
    tasks = data.tasks || [];
    currentTemplate = data.currentTemplate;
  } catch (error) {
    console.error('Failed to load tasks:', error);
  }
}

async function saveTasks() {
  try {
    await ipcRenderer.invoke('save-tasks', {
      tasks: tasks,
      currentTemplate: currentTemplate
    });
  } catch (error) {
    console.error('Failed to save tasks:', error);
  }
}

async function loadTemplates() {
  try {
    templates = await ipcRenderer.invoke('load-templates');
  } catch (error) {
    console.error('Failed to load templates:', error);
    templates = [];
  }
}

async function saveTemplates() {
  try {
    await ipcRenderer.invoke('save-templates', templates);
  } catch (error) {
    console.error('Failed to save templates:', error);
  }
}

// Settings persistence
async function saveSettingsData() {
  try {
    await ipcRenderer.invoke('save-settings', settings);
  } catch (error) {
    console.error('Failed to save settings:', error);
  }
}

// Utility functions
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
    if (confirm('Close Echoesight Overlay completely?')) {
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

// Event listeners and initialization
document.addEventListener('DOMContentLoaded', () => {
  // Task input event listeners
  document.getElementById('taskInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter') {
      addTask();
    }
  });

  document.getElementById('importTemplateInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter' && e.ctrlKey) {
      importTemplate();
    }
  });

  // Subtask input event listeners
  document.getElementById('subTaskInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter') {
      saveSubTask();
    }
  });

  // Global keydown listener for hotkey recording
  document.addEventListener('keydown', handleHotkeyRecording);

  document.getElementById('taskInput').addEventListener('focus', function () {
    console.log('Task input gained focus');
  });

  document.getElementById('taskInput').addEventListener('blur', function () {
    console.log('Task input lost focus');
  });

  document.getElementById('templateNameInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter') {
      saveTemplate();
    }
  });

  document.getElementById('importTemplateInput').addEventListener('keypress', function (e) {
    if (e.key === 'Enter' && e.ctrlKey) {
      importTemplate();
    }
  });

  // Global keydown listener for hotkey recording
  document.addEventListener('keydown', handleHotkeyRecording);

  // Click outside modal to stop recording
  document.addEventListener('click', function (e) {
    if (window.recordingHotkey && !e.target.closest('.modal-content')) {
      stopRecording();
    }
  });

  // Make header draggable
  const header = document.getElementById('header');
  let isDragging = false;
  let dragOffset = { x: 0, y: 0 };

  header.addEventListener('mousedown', (e) => {
    try {
      console.log('Header mousedown');
      isDragging = true;
      const rect = header.getBoundingClientRect();
      dragOffset.x = e.clientX - rect.left;
      dragOffset.y = e.clientY - rect.top;
    } catch (error) {
      console.error('Error in mousedown:', error);
    }
  });

  document.addEventListener('mousemove', (e) => {
    try {
      if (isDragging) {
        const newX = e.screenX - dragOffset.x;
        const newY = e.screenY - dragOffset.y;

        // Send move request to main process
        ipcRenderer.send('move-window', { x: newX, y: newY });
      }
    } catch (error) {
      console.error('Error in mousemove:', error);
      isDragging = false;
    }
  });

  document.addEventListener('mouseup', () => {
    try {
      isDragging = false;
    } catch (error) {
      console.error('Error in mouseup:', error);
    }
  });

  // Debug: Log when overlay gets focus/blur
  window.addEventListener('focus', () => {
    console.log('Overlay gained focus');
  });

  window.addEventListener('blur', () => {
    console.log('Overlay lost focus');
  });

  // Debug function - call from console to test input
  window.testInput = function () {
    const input = document.getElementById('taskInput');
    input.focus();
    input.value = 'Test task';
    console.log('Test input set. Try typing or clicking Add.');
  };

  // Debug function - call from console to test buttons
  window.testMinimize = function () {
    console.log('Testing minimize...');
    minimizeOverlay();
  };

  // Global error handler
  window.addEventListener('error', function (e) {
    console.error('Global error:', e.error);
    console.error('Error message:', e.message);
    console.error('Error location:', e.filename, e.lineno, e.colno);
  });

  window.addEventListener('unhandledrejection', function (e) {
    console.error('Unhandled promise rejection:', e.reason);
  });

  // Initialize the app
  initializeApp();
});

// make functions global for onclick handlers
window.addTask = addTask;
window.toggleTask = toggleTask;
window.deleteTask = deleteTask;
window.showContextMenu = showContextMenu;
window.addSubTask = addSubTask;
window.clearAllTasks = clearAllTasks;
window.closeSubTaskModal = closeSubTaskModal;
window.saveSubTask = saveSubTask;
window.minimizeOverlay = minimizeOverlay;
window.closeOverlay = closeOverlay;
window.toggleInteractiveMode = toggleInteractiveMode;
window.showSettingsModal = showSettingsModal;
window.closeSettingsModal = closeSettingsModal;
window.resetHotkeys = resetHotkeys;
window.updateTransparency = updateTransparency;
window.updateTheme = updateTheme;
window.recordHotkey = recordHotkey;
window.resetPosition = resetPosition;
window.saveSettings = saveSettings;
window.showSaveTemplateModal = showSaveTemplateModal;
window.closeSaveTemplateModal = closeSaveTemplateModal;
window.saveTemplate = saveTemplate;
window.loadTemplate = loadTemplate;
window.deleteTemplate = deleteTemplate;
window.exportTemplate = exportTemplate;
window.showImportModal = showImportModal;
window.closeImportModal = closeImportModal;
window.importTemplate = importTemplate;
window.handleFileImport = handleFileImport;
window.loadCommunityTemplates = loadCommunityTemplates;
window.closeCommunityModal = closeCommunityModal;
window.importCommunityTemplate = importCommunityTemplate;
window.previewCommunityTemplate = previewCommunityTemplate;
window.openThemesFolder = openThemesFolder;
window.reloadThemes = reloadThemes;
window.showThemesPath = showThemesPath;