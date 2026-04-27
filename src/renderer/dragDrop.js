export function createDragDrop(ctx) {
  const { state } = ctx;
  const { tasks } = state;

  function renderTasks() {
    ctx.managers.ui.renderTasks();
  }

  function saveTasks() {
    return ctx.managers.task.saveTasks();
  }

let draggedTaskId = null;
let draggedElement = null;
let placeholder = null;
let lastOperation = null;

// Initialize drag and drop functionality
function initializeDragAndDrop() {
  const taskItems = document.querySelectorAll('.task-item');
  
  taskItems.forEach(taskItem => {
    const taskId = parseInt(taskItem.dataset.taskId);
    
    // Make task draggable
    taskItem.draggable = true;
    taskItem.style.cursor = 'grab';
    
    // Add drag event listeners (keep dragover for visual feedback)
    taskItem.addEventListener('dragstart', handleDragStart);
    taskItem.addEventListener('dragend', handleDragEnd);
    taskItem.addEventListener('dragover', handleDragOver);
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
    background: rgba(212, 175, 55, 0.2);
    border: 2px dashed #d4af37;
    border-radius: 4px;
    margin: 2px 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: #d4af37;
    font-size: 11px;
    font-style: italic;
    list-style: none;
    transition: all 0.2s ease;
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
  lastOperation = null;
}

// Handle drag over
function handleDragOver(e) {
  e.preventDefault();
  e.stopPropagation();
  e.dataTransfer.dropEffect = 'move';
  
  const targetElement = e.currentTarget;
  if (targetElement === draggedElement) return;
  
  // PG: Use shared logic to calculate drop operation
  const operation = calculateDropOperation(e, draggedTaskId, targetElement);
  if (!operation) return;
  
  
  // PG: Aids to have realtime Drop Handler. Check if operation has actually changed
  const operationKey = `${operation.operationType}-${operation.isAbove}-${operation.targetTaskId}`;
  if (lastOperation === operationKey) {
    return; // No change, don't recreate placeholder
  }
  lastOperation = operationKey;
  
  // PG: Remove existing placeholder
  if (placeholder && placeholder.parentNode) {
    placeholder.parentNode.removeChild(placeholder);
    placeholder = null;
  }
  
  // PG: Create a fresh placeholder
  placeholder = document.createElement('li');
  placeholder.className = 'drag-placeholder';
  placeholder.style.cssText = `
    height: ${targetElement.offsetHeight}px;
    border: 2px dashed;
    border-radius: 4px;
    margin: 2px 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-style: italic;
    list-style: none;
    pointer-events: none;
    user-select: none;
  `;
  
  // PG: Operation Styles
  if (operation.makeSubtask) {
    placeholder.style.borderColor = '#4285f4';
    placeholder.style.color = '#4285f4';
    placeholder.style.background = 'rgba(66, 133, 244, 0.1)';
    placeholder.textContent = 'Drop to make subtask';
    placeholder.style.marginLeft = `${(operation.targetInfo.level + 1) * 20}px`;
  } else if (operation.operationType.includes('Promote')) {
    placeholder.style.borderColor = '#32cd32';
    placeholder.style.color = '#32cd32';
    placeholder.style.background = 'rgba(50, 205, 50, 0.1)';
    placeholder.textContent = operation.operationType;
  } else {
    placeholder.style.borderColor = '#d4af37';
    placeholder.style.color = '#d4af37';
    placeholder.style.background = 'rgba(212, 175, 55, 0.1)';
    placeholder.textContent = operation.operationType;
  }
  
  // PG: Insert placeholder in the correct position
  if (operation.makeSubtask) {
    // PG: Insert after target for subtask creation
    targetElement.parentNode.insertBefore(placeholder, targetElement.nextSibling);
  } else {
    // PG: Insert above or below for reordering
    if (operation.isAbove) {
      targetElement.parentNode.insertBefore(placeholder, targetElement);
    } else {
      targetElement.parentNode.insertBefore(placeholder, targetElement.nextSibling);
    }
  }
}

// Handle drop
function handleDrop(e) {
  e.preventDefault();
  console.log('Drop triggered');
  
  if (!draggedTaskId) return;
  
  const targetTaskId = parseInt(e.currentTarget.dataset.taskId);
  if (draggedTaskId === targetTaskId) return;
  
  const draggedInfo = findTaskWithParent(draggedTaskId);
  const targetInfo = findTaskWithParent(targetTaskId);
  
  if (!draggedInfo || !targetInfo) return;
  
  const targetElement = e.currentTarget;
  const rect = targetElement.getBoundingClientRect();
  const midpoint = rect.top + rect.height / 2;
  const isAbove = e.clientY < midpoint;
  const leftThird = rect.left + rect.width / 3;
  const isIndented = e.clientX > leftThird;
  
  let makeSubtask = false;
  
  if (isIndented && !isAbove && draggedInfo.level <= targetInfo.level) {
    makeSubtask = true;
  }
  
  reorderTasksAdvanced(draggedTaskId, targetTaskId, isAbove, makeSubtask);
}

function handleDragEnter(e) {
  e.preventDefault();
  e.stopPropagation();
  if (e.currentTarget !== draggedElement) {
    e.currentTarget.style.backgroundColor = 'rgba(212, 175, 55, 0.1)';
  }
}

function handleDragLeave(e) {
  if (e.currentTarget !== draggedElement) {
    e.currentTarget.style.backgroundColor = '';
  }
}

// Helper function to calculate drop operation details
function calculateDropOperation(e, draggedTaskId, targetElement) {
  if (!draggedTaskId || !targetElement) return null;
  
  const draggedInfo = findTaskWithParent(draggedTaskId);
  const targetTaskId = parseInt(targetElement.dataset.taskId);
  const targetInfo = findTaskWithParent(targetTaskId);
  
  if (!draggedInfo || !targetInfo) return null;
  
  // Calculate drop details from mouse position
  const rect = targetElement.getBoundingClientRect();
  const midpoint = rect.top + rect.height / 2;
  const isAbove = e.clientY < midpoint;
  
  // Balanced indentation detection - right half of the task
  const rightHalf = rect.left + rect.width * 0.5;
  const isIndented = e.clientX > rightHalf;
  
  // Create subtask when dropping in right half, below target, and valid level
  let makeSubtask = false;
  if (isIndented && !isAbove && draggedInfo.level <= targetInfo.level) {
    makeSubtask = true;
  }
  
  // Determine operation type for visual feedback
  let operationType = '';
  let placeholderColor = '#d4af37';
  
  if (makeSubtask) {
    operationType = 'Make subtask of target';
    placeholderColor = '#4285f4';
  } else if (draggedInfo.level === targetInfo.level) {
    operationType = 'Reorder within same level';
    placeholderColor = '#d4af37';
  } else if (draggedInfo.level > targetInfo.level) {
    operationType = 'Promote to higher level';
    placeholderColor = '#32cd32';
  } else {
    operationType = 'Reorder as sibling';
    placeholderColor = '#d4af37';
  }
  
  return {
    isAbove,
    isIndented,
    makeSubtask,
    operationType,
    placeholderColor,
    draggedInfo,
    targetInfo,
    targetTaskId
  };
}

// Advanced reorder function with subtask conversion support
function reorderTasksAdvanced(draggedId, targetId, insertAbove, makeSubtask = false) {
  console.log('Advanced reordering:', { draggedId, targetId, insertAbove, makeSubtask });

  // Find both tasks in the hierarchical structure
  const draggedInfo = findTaskWithParent(draggedId);
  const targetInfo = findTaskWithParent(targetId);

  if (!draggedInfo || !targetInfo) {
    console.log('Task not found:', !draggedInfo ? draggedId : targetId);
    return;
  }

  // Prevent cross-list drops (main <-> background)
  const draggedMode = draggedInfo.task.mode || 'main';
  const targetMode = targetInfo.task.mode || 'main';
  if (draggedMode !== targetMode) {
    console.log('Cannot reorder across main/background lists');
    return;
  }

  console.log('Dragged task:', draggedInfo.task.text, 'at level', draggedInfo.level);
  console.log('Target task:', targetInfo.task.text, 'at level', targetInfo.level);
  console.log('Dragged parent:', draggedInfo.parent ? draggedInfo.parent.text : 'none');
  console.log('Target parent:', targetInfo.parent ? targetInfo.parent.text : 'none');
  
  // Remove dragged task from its current location
  const draggedTask = draggedInfo.task;
  removeTaskFromParent(draggedId);
  
  // Determine the new parent and position
  let newParentArray, insertIndex;
  let operation = '';
  
  if (makeSubtask) {
    // Make the dragged task a subtask of the target
    const targetTask = targetInfo.task;
    if (!targetTask.children) targetTask.children = [];
    newParentArray = targetTask.children;
    insertIndex = newParentArray.length; // Add at end
    operation = `Made "${draggedTask.text}" a subtask of "${targetTask.text}"`;
    
  } else if (draggedInfo.level === targetInfo.level) {
    // Same level - reorder within same parent
    if (targetInfo.parent) {
      newParentArray = targetInfo.parent.children;
      const targetIndex = newParentArray.findIndex(t => t.id === targetId);
      insertIndex = insertAbove ? targetIndex : targetIndex + 1;
    } else {
      // Top level
      newParentArray = tasks;
      const targetIndex = newParentArray.findIndex(t => t.id === targetId);
      insertIndex = insertAbove ? targetIndex : targetIndex + 1;
    }
    operation = `Reordered "${draggedTask.text}" within same level`;
    
  } else if (draggedInfo.level > targetInfo.level) {
    // Dragging subtask to higher level - promote to same level as target
    if (targetInfo.parent) {
      // Target has a parent, so promote to be sibling of target
      newParentArray = targetInfo.parent.children;
      const targetIndex = newParentArray.findIndex(t => t.id === targetId);
      insertIndex = insertAbove ? targetIndex : targetIndex + 1;
    } else {
      // Target is top level, so promote to top level
      newParentArray = tasks;
      const targetIndex = newParentArray.findIndex(t => t.id === targetId);
      insertIndex = insertAbove ? targetIndex : targetIndex + 1;
    }
    operation = `Promoted "${draggedTask.text}" from level ${draggedInfo.level} to level ${targetInfo.level}`;
    
  } else {
    // Dragging higher level task to lower level - make it a sibling of target
    if (targetInfo.parent) {
      newParentArray = targetInfo.parent.children;
      const targetIndex = newParentArray.findIndex(t => t.id === targetId);
      insertIndex = insertAbove ? targetIndex : targetIndex + 1;
    } else {
      // Same level at top
      newParentArray = tasks;
      const targetIndex = newParentArray.findIndex(t => t.id === targetId);
      insertIndex = insertAbove ? targetIndex : targetIndex + 1;
    }
    operation = `Moved "${draggedTask.text}" to be sibling of "${targetInfo.task.text}"`;
  }
  
  // Insert the task at the new position
  newParentArray.splice(insertIndex, 0, draggedTask);
  
  console.log('Operation completed:', operation);
  showReorderFeedback(operation);
  renderTasks();
  saveTasks();
}

// Legacy function for backward compatibility
function reorderTasks(draggedId, targetId, insertAbove) {
  reorderTasksAdvanced(draggedId, targetId, insertAbove, false);
}

// Helper function to find a task and its parent info
function findTaskWithParent(taskId, taskArray = tasks, parent = null, level = 0) {
  for (const task of taskArray) {
    if (task.id === taskId) {
      return { task, parent, level, array: taskArray };
    }
    
    if (task.children && task.children.length > 0) {
      const result = findTaskWithParent(taskId, task.children, task, level + 1);
      if (result) return result;
    }
  }
  return null;
}

// Helper function to remove a task from its current parent
function removeTaskFromParent(taskId) {
  function removeFromArray(taskArray) {
    for (let i = 0; i < taskArray.length; i++) {
      if (taskArray[i].id === taskId) {
        taskArray.splice(i, 1);
        return true;
      }
      
      if (taskArray[i].children && removeFromArray(taskArray[i].children)) {
        return true;
      }
    }
    return false;
  }
  
  return removeFromArray(tasks);
}

// Show feedback when task order is updated
function showReorderFeedback(message = 'Task order updated!') {
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
    max-width: 300px;
    word-wrap: break-word;
  `;
  feedback.textContent = message;
  
  // Add slide-in animation if not already present
  if (!document.getElementById('slideInStyle')) {
    const style = document.createElement('style');
    style.id = 'slideInStyle';
    style.textContent = `
      @keyframes slideIn {
        from { transform: translateX(100%); opacity: 0; }
        to { transform: translateX(0); opacity: 1; }
      }
    `;
    document.head.appendChild(style);
  }
  
  document.body.appendChild(feedback);
  
  // Remove after 3 seconds
  setTimeout(() => {
    if (feedback.parentNode) {
      feedback.style.animation = 'slideIn 0.3s ease reverse';
      setTimeout(() => feedback.remove(), 300);
    }
  }, 3000);
}

// PG: Update placeholder based on operation, this was buggy asf so check it thoroughly
function updatePlaceholder(operation, targetElement) {
  // Remove existing placeholder
  if (placeholder && placeholder.parentNode) {
    placeholder.parentNode.removeChild(placeholder);
    placeholder = null;
  }
  
  // Create a fresh placeholder
  placeholder = document.createElement('li');
  placeholder.className = 'drag-placeholder';
  placeholder.style.cssText = `
    height: ${targetElement.offsetHeight}px;
    border: 2px dashed;
    border-radius: 4px;
    margin: 2px 0;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 11px;
    font-style: italic;
    list-style: none;
    pointer-events: none;
    user-select: none;
  `;
  
  if (operation.makeSubtask) {
    placeholder.style.borderColor = '#4285f4';
    placeholder.style.color = '#4285f4';
    placeholder.style.background = 'rgba(66, 133, 244, 0.1)';
    placeholder.textContent = 'Drop to make subtask';
    placeholder.style.marginLeft = `${(operation.targetInfo.level + 1) * 20}px`;
  } else if (operation.operationType.includes('Promote')) {
    placeholder.style.borderColor = '#32cd32';
    placeholder.style.color = '#32cd32';
    placeholder.style.background = 'rgba(50, 205, 50, 0.1)';
    placeholder.textContent = operation.operationType;
  } else {
    placeholder.style.borderColor = '#d4af37';
    placeholder.style.color = '#d4af37';
    placeholder.style.background = 'rgba(212, 175, 55, 0.1)';
    placeholder.textContent = operation.operationType;
  }
  
  if (operation.makeSubtask) {
    // PG: Insert after target for subtask creation
    targetElement.parentNode.insertBefore(placeholder, targetElement.nextSibling);
  } else {
    // PG: Insert above or below for reordering
    if (operation.isAbove) {
      targetElement.parentNode.insertBefore(placeholder, targetElement);
    } else {
      targetElement.parentNode.insertBefore(placeholder, targetElement.nextSibling);
    }
  }
}

// PG: Add container-level drop handler, it was per Task before but I coudn't get it to work properly
function setupContainerDropHandler() {
  const taskList = document.getElementById('taskList');
  if (!taskList.hasDropHandler) {
    taskList.addEventListener('drop', (e) => {
      e.preventDefault();
      console.log('Container drop event fired!');
      console.log('Drop target element:', e.target);
      console.log('Dragged task ID:', draggedTaskId);
      
      // PG: Find the task item that was dropped on
      let target = e.target.closest('.task-item');
      
      // PG: If we dropped on placeholder, find the adjacent task
      if (!target && e.target.classList.contains('drag-placeholder')) {
        console.log('Dropped on placeholder, finding adjacent task');
        const placeholder = e.target;
        
        // PG: Find the task before or after the placeholder
        let adjacentTask = placeholder.previousElementSibling;
        if (!adjacentTask || !adjacentTask.classList.contains('task-item')) {
          adjacentTask = placeholder.nextElementSibling;
        }
        
        if (adjacentTask && adjacentTask.classList.contains('task-item')) {
          target = adjacentTask;
          console.log('Found adjacent task:', target.dataset.taskId);
        }
      }
      
      // PG: If still no target, find the closest task item to the drop position
      if (!target) {
        console.log('No direct target, searching for closest task item');
        const taskItems = document.querySelectorAll('.task-item');
        let closestTask = null;
        let closestDistance = Infinity;
        
        taskItems.forEach(task => {
          const rect = task.getBoundingClientRect();
          const distance = Math.abs(e.clientY - (rect.top + rect.height / 2));
          if (distance < closestDistance) {
            closestDistance = distance;
            closestTask = task;
          }
        });
        
        if (closestTask) {
          target = closestTask;
          console.log('Found closest task:', target.dataset.taskId);
        }
      }
      
      console.log('Found task item target:', target);
      console.log('Target dataset:', target ? target.dataset : 'no target');
      
      if (target && target.dataset.taskId && draggedTaskId) {
        const targetTaskId = parseInt(target.dataset.taskId);
        console.log('Dropping task', draggedTaskId, 'onto task', targetTaskId);
        
        if (draggedTaskId !== targetTaskId) {
          // Use shared logic to calculate drop operation
          const operation = calculateDropOperation(e, draggedTaskId, target);
          if (operation) {
            console.log('Drop operation details:', {
              isAbove: operation.isAbove, 
              isIndented: operation.isIndented, 
              makeSubtask: operation.makeSubtask,
              operationType: operation.operationType,
              draggedLevel: operation.draggedInfo.level,
              targetLevel: operation.targetInfo.level
            });
            
            reorderTasksAdvanced(draggedTaskId, operation.targetTaskId, operation.isAbove, operation.makeSubtask);
          }
        }
      } else {
        console.log('Drop validation failed:', {
          hasTarget: !!target,
          hasTaskId: target ? !!target.dataset.taskId : false,
          hasDraggedId: !!draggedTaskId
        });
      }
    });
    
    taskList.addEventListener('dragover', (e) => {
      e.preventDefault();
      
      // PG: Find the actual task item being hovered over
      let target = e.target.closest('.task-item');
      if (!target) {
        // PG: As above, so below
        const taskItems = document.querySelectorAll('.task-item');
        let closestTask = null;
        let closestDistance = Infinity;
        
        taskItems.forEach(task => {
          const rect = task.getBoundingClientRect();
          const distance = Math.abs(e.clientY - (rect.top + rect.height / 2));
          if (distance < closestDistance && distance < 50) { // Within 50px
            closestDistance = distance;
            closestTask = task;
          }
        });
        
        if (closestTask) {
          target = closestTask;
        }
      }
      
      if (target && draggedTaskId) {
        const operation = calculateDropOperation(e, draggedTaskId, target);
        if (operation) {
          
          const operationKey = `${operation.operationType}-${operation.isAbove}-${operation.targetTaskId}`;
          if (lastOperation !== operationKey) {
            lastOperation = operationKey;
            updatePlaceholder(operation, target);
          }
        }
      }
    });
    
    taskList.hasDropHandler = true;
  }

  // Also set up drop handler for background task list
  const bgTaskList = document.getElementById('backgroundTaskList');
  if (bgTaskList && !bgTaskList.hasDropHandler) {
    bgTaskList.addEventListener('drop', (e) => {
      e.preventDefault();
      let target = e.target.closest('.task-item');
      if (!target && e.target.classList.contains('drag-placeholder')) {
        const ph = e.target;
        let adjacentTask = ph.previousElementSibling;
        if (!adjacentTask || !adjacentTask.classList.contains('task-item')) {
          adjacentTask = ph.nextElementSibling;
        }
        if (adjacentTask && adjacentTask.classList.contains('task-item')) {
          target = adjacentTask;
        }
      }
      if (target && target.dataset.taskId && draggedTaskId) {
        const targetTaskId = parseInt(target.dataset.taskId);
        if (draggedTaskId !== targetTaskId) {
          const operation = calculateDropOperation(e, draggedTaskId, target);
          if (operation) {
            reorderTasksAdvanced(draggedTaskId, operation.targetTaskId, operation.isAbove, operation.makeSubtask);
          }
        }
      }
    });

    bgTaskList.addEventListener('dragover', (e) => {
      e.preventDefault();
      let target = e.target.closest('.task-item');
      if (target && draggedTaskId) {
        const operation = calculateDropOperation(e, draggedTaskId, target);
        if (operation) {
          const operationKey = `${operation.operationType}-${operation.isAbove}-${operation.targetTaskId}`;
          if (lastOperation !== operationKey) {
            lastOperation = operationKey;
            updatePlaceholder(operation, target);
          }
        }
      }
    });

    bgTaskList.hasDropHandler = true;
  }
}


  return {
    initializeDragAndDrop,
    handleDragStart,
    handleDragEnd,
    handleDragOver,
    handleDrop,
    handleDragEnter,
    handleDragLeave,
    calculateDropOperation,
    reorderTasksAdvanced,
    reorderTasks,
    findTaskWithParent,
    removeTaskFromParent,
    showReorderFeedback,
    updatePlaceholder,
    setupContainerDropHandler
  };
}
