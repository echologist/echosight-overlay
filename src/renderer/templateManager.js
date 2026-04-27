export function createTemplateManager(ctx) {
  const { ipcRenderer, state } = ctx;
  const { tasks, templates } = state;

function replaceArray(target, values) {
  target.splice(0, target.length, ...values);
}

  function renderTasks() {
    ctx.managers.ui.renderTasks();
  }

  function updateProgress() {
    ctx.managers.progress.updateProgress();
  }

  function saveTasks() {
    return ctx.managers.task.saveTasks();
  }

  function migrateTaskStructure() {
    ctx.managers.task.migrateTaskStructure();
  }

  function escapeHtml(text) {
    return ctx.managers.ui.escapeHtml(text);
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

  // Build index map: task ID -> positional index for trigger remapping
  const taskIndexMap = {};
  let flatIndex = 0;
  function buildIndexMap(taskArray) {
    taskArray.forEach(task => {
      taskIndexMap[task.id] = flatIndex++;
      if (task.children) buildIndexMap(task.children);
    });
  }
  buildIndexMap(tasks.filter(t => !t.completed));

  function stripTaskForTemplate(task) {
    const stripped = { text: task.text };
    if (task.children && task.children.length > 0) {
      stripped.children = task.children.filter(c => !c.completed).map(c => stripTaskForTemplate(c));
    }
    if (task.mode === 'background') {
      stripped.mode = 'background';
      stripped.backgroundOptions = task.backgroundOptions;
    }
    if (task.triggers && task.triggers.length > 0) {
      stripped.triggerIndices = task.triggers
        .map(id => taskIndexMap[id])
        .filter(idx => idx !== undefined);
    }
    return stripped;
  }

  const template = {
    id: Date.now(),
    name: name,
    tasks: tasks.filter(t => !t.completed).map(t => stripTaskForTemplate(t)),
    createdAt: new Date().toISOString()
  };

  // Remove existing template with same name
  replaceArray(templates, templates.filter(t => t.name !== name));
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
  replaceArray(tasks, template.tasks.map((t, index) => ({
    id: Date.now() + index,
    text: t.text,
    completed: false,
    createdAt: new Date().toISOString(),
    children: t.children || [],
    mode: t.mode || 'main',
    triggers: [],
    activated: t.mode === 'background' ? false : true,
    activatedAt: null,
    backgroundOptions: t.backgroundOptions || null
  })));

  // Remap trigger references from template positional indices to new IDs
  if (template.tasks.some(t => t.triggerIndices && t.triggerIndices.length > 0)) {
    remapTemplateTriggersOnLoad(template.tasks, tasks);
  }

  state.currentTemplate = template.name;
  migrateTaskStructure();
  renderTasks();
  updateProgress();
  saveTasks();
  
  // Force window focus after loading template
  setTimeout(() => {
    if (window.focus) {
      window.focus();
    }
    // Also try to focus the main window via IPC
    try {
      ipcRenderer.send('focus-window');
    } catch (error) {
      console.log('Could not focus window via IPC');
    }
  }, 200);
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
    replaceArray(templates, templates.filter(t => t.id !== templateId));

    // Save updated templates
    await saveTemplates();

    // Update dropdown
    updateTemplateSelect();

    alert(`Template "${template.name}" deleted successfully!`);
  }
}

// Security functions for template import
function isValidTemplateInput(input) {
  // Check for potentially dangerous patterns
  const dangerousPatterns = [
    /<script/i,
    /javascript:/i,
    /on\w+\s*=/i,
    /eval\s*\(/i,
    /Function\s*\(/i,
    /setTimeout\s*\(/i,
    /setInterval\s*\(/i,
    /document\./i,
    /window\./i,
    /localStorage/i,
    /sessionStorage/i,
    /cookie/i,
    /location\./i,
    /navigator\./i
  ];

  // Reject if any dangerous patterns found
  for (const pattern of dangerousPatterns) {
    if (pattern.test(input)) {
      return false;
    }
  }

  // Only allow JSON-like input (starts with { and ends with })
  return input.trim().startsWith('{') && input.trim().endsWith('}');
}

function sanitizeTemplateData(data) {
  // Create a clean copy with only allowed properties
  const sanitized = {
    name: sanitizeString(data.name) || 'Imported Template',
    tasks: []
  };

  // Sanitize tasks array
  if (Array.isArray(data.tasks)) {
    sanitized.tasks = data.tasks
      .filter(task => typeof task === 'string' || (typeof task === 'object' && task.text))
      .map(task => {
        if (typeof task === 'string') {
          return sanitizeString(task);
        } else if (typeof task === 'object' && task.text) {
          return sanitizeString(task.text);
        }
        return '';
      })
      .filter(task => task.length > 0 && task.length <= 200); // Limit task length
  }

  return sanitized;
}

function sanitizeString(str) {
  if (typeof str !== 'string') return '';
  
  return str
    .replace(/[<>]/g, '') // Remove HTML brackets
    .replace(/javascript:/gi, '') // Remove javascript: protocol
    .replace(/on\w+\s*=/gi, '') // Remove event handlers
    .trim()
    .substring(0, 200); // Limit string length
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

    // Check if template has background tasks
    const hasBackgroundTasks = template.tasks.some(t => t.mode === 'background');

    // Create exportable template data
    const exportData = {
      name: template.name,
      tasks: template.tasks,
      version: hasBackgroundTasks ? "2.0" : "1.0",
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
    // Validate input before parsing
    if (!isValidTemplateInput(inputData)) {
      throw new Error('Invalid template format. Only JSON templates are supported.');
    }

    // Safe JSON parsing with size limit
    if (inputData.length > 100000) { // 100KB limit
      throw new Error('Template data too large');
    }

    // Try to parse as JSON first
    if (inputData.startsWith('{')) {
      // It's JSON
      templateData = JSON.parse(inputData);

      // Validate JSON template structure
      if (!templateData.name || !templateData.tasks || !Array.isArray(templateData.tasks)) {
        throw new Error('Invalid JSON template format');
      }

      // Sanitize template data
      templateData = sanitizeTemplateData(templateData);
    } else {
      // It might be a share code, but we'll just try JSON parsing
      templateData = JSON.parse(inputData);
      
      // Sanitize after parsing
      templateData = sanitizeTemplateData(templateData);
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
      tasks: templateData.tasks.map(task => {
        if (typeof task === 'string') {
          return { text: task, children: [] };
        }
        // Preserve background task metadata from v2.0 imports
        const t = { text: task.text || task, children: task.children || [] };
        if (task.mode) t.mode = task.mode;
        if (task.backgroundOptions) t.backgroundOptions = task.backgroundOptions;
        if (task.triggerIndices) t.triggerIndices = task.triggerIndices;
        return t;
      }),
      createdAt: new Date().toISOString(),
      imported: true
    };

    // Remove existing template with same name
    replaceArray(templates, templates.filter(t => t.name !== templateData.name));
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

// Remap trigger indices from template format back to live task IDs
function remapTemplateTriggersOnLoad(templateTasks, loadedTasks) {
  // Build flat array of loaded tasks to map positional index -> task ID
  const flatLoaded = [];
  function flatten(taskArray) {
    taskArray.forEach(task => {
      flatLoaded.push(task);
      if (task.children) flatten(task.children);
    });
  }
  flatten(loadedTasks);

  // Also flatten template tasks to find triggerIndices
  const flatTemplate = [];
  function flattenTemplate(taskArray) {
    taskArray.forEach(task => {
      flatTemplate.push(task);
      if (task.children) flattenTemplate(task.children);
    });
  }
  flattenTemplate(templateTasks);

  // Remap triggers
  flatTemplate.forEach((tmplTask, i) => {
    if (tmplTask.triggerIndices && tmplTask.triggerIndices.length > 0 && flatLoaded[i]) {
      flatLoaded[i].triggers = tmplTask.triggerIndices
        .map(idx => flatLoaded[idx]?.id)
        .filter(Boolean);
    }
  });
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
      tasks: template.tasks.map(task => ({ 
        text: task,
        children: []
      })),
      createdAt: new Date().toISOString(),
      community: true
    };

    // Remove existing template with same name
    replaceArray(templates, templates.filter(t => t.name !== template.name));
    templates.push(newTemplate);

    await saveTemplates();
    updateTemplateSelect();

    alert(`Community template "${template.name}" added to your templates!`);
    
    // Force window focus after importing
    setTimeout(() => {
      if (window.focus) {
        window.focus();
      }
      try {
        ipcRenderer.send('focus-window');
      } catch (error) {
        console.log('Could not focus window via IPC');
      }
    }, 200);
    
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


  async function loadTemplates() {
    try {
      replaceArray(templates, await ipcRenderer.invoke('load-templates'));
    } catch (error) {
      console.error('Failed to load templates:', error);
      replaceArray(templates, []);
    }
  }

  async function saveTemplates() {
    try {
      await ipcRenderer.invoke('save-templates', templates);
    } catch (error) {
      console.error('Failed to save templates:', error);
    }
  }

  return {
    showSaveTemplateModal,
    closeSaveTemplateModal,
    saveTemplate,
    loadTemplate,
    deleteTemplate,
    isValidTemplateInput,
    sanitizeTemplateData,
    sanitizeString,
    exportTemplate,
    showImportModal,
    closeImportModal,
    handleFileImport,
    importTemplate,
    remapTemplateTriggersOnLoad,
    updateTemplateSelect,
    getCommunityTemplates,
    loadCommunityTemplates,
    closeCommunityModal,
    importCommunityTemplate,
    previewCommunityTemplate,
    loadTemplates,
    saveTemplates
  };
}
