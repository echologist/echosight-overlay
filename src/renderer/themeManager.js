export function createThemeManager(ctx) {
  const { ipcRenderer, state } = ctx;
  const { themes, settings } = state;

function replaceArray(target, values) {
  target.splice(0, target.length, ...values);
}

  function updateInteractiveVisuals(interactive) {
    ctx.managers.ui.updateInteractiveVisuals(interactive);
  }

function resetHotkeys() {
  if (confirm('Reset hotkeys to default values (Ctrl+Shift+T, Ctrl+Shift+I, Ctrl+Shift+N, and Ctrl+Shift+Z)?')) {
    settings.hotkeys.toggleVisibility = 'Ctrl+Shift+T';
    settings.hotkeys.toggleInteractive = 'Ctrl+Shift+I';
    settings.hotkeys.completeNextTask = 'Ctrl+Shift+N';
    settings.hotkeys.undoLastAction = 'Ctrl+Shift+Z';

    // Update the input fields
    document.getElementById('toggleVisibilityHotkey').value = 'Ctrl+Shift+T';
    document.getElementById('toggleInteractiveHotkey').value = 'Ctrl+Shift+I';
    document.getElementById('completeNextTaskHotkey').value = 'Ctrl+Shift+N';
    document.getElementById('undoLastActionHotkey').value = 'Ctrl+Shift+Z';

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
  document.getElementById('undoLastActionHotkey').value = settings.hotkeys.undoLastAction;
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

function updateTransparencyControls(supportsTransparency) {
  const slider = document.getElementById('transparencySlider');
  const label = document.querySelector('label[for="transparencySlider"]');
  const valueDisplay = document.getElementById('transparencyValue');
  
  if (supportsTransparency) {
    slider.disabled = false;
    slider.style.opacity = '1';
    slider.style.cursor = 'pointer';
    if (label) {
      label.style.opacity = '1';
      label.style.color = '';
    }
    if (valueDisplay) {
      valueDisplay.style.opacity = '1';
      valueDisplay.style.color = '';
      // Reset text to show current transparency value
      valueDisplay.textContent = settings.transparency + '% visible';
    }
  } else {
    slider.disabled = true;
    slider.style.opacity = '0.5';
    slider.style.cursor = 'not-allowed';
    if (label) {
      label.style.opacity = '0.5';
      label.style.color = '#666';
    }
    if (valueDisplay) {
      valueDisplay.style.opacity = '0.5';
      valueDisplay.style.color = '#666';
      valueDisplay.textContent = 'Not supported by this theme';
    }
  }
}

function updateTransparency(value) {
  document.getElementById('transparencyValue').textContent = value + '% visible';

  // Apply transparency preview immediately
  settings.transparency = parseInt(value);
  
  // Update the CSS variable for theme usage (keep as 0-1 range)
  const opacity = value / 100;
  document.documentElement.style.setProperty('--user-transparency', opacity);
  
  // Theme now handles transparency properly, no override needed
  
  applyTransparencySettings();
}

async function updateTheme(themeId) {
  console.log('Theme changed to:', themeId);
  settings.theme = themeId;
  
  // PG: Check if theme supports transparency and update UI
  const theme = await getCurrentTheme();
  updateTransparencyControls(theme?.supportsTransparency !== false);
  
  applyTheme();

  if (state.isInteractiveMode) {
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
    
    // Load theme assets first
    await loadThemeAssets(theme);
    
    const style = document.createElement('style');
    style.id = 'theme-style';
    
    let css = await generateThemeCSS(theme);
    console.log('Generated CSS with transparency:', css.substring(0, 500) + '...');
    style.textContent = css;
    document.head.appendChild(style);
    
    applyFonts(theme);
    
    // PG: Apply current transparency settings as CSS variable
    const opacity = settings.transparency / 100;
    document.documentElement.style.setProperty('--user-transparency', opacity);
    
    updateTransparencyControls(theme?.supportsTransparency !== false);
    
  } catch (error) {
    console.error('Failed to apply theme:', error);
  }
}

function removeExistingStyles() {
  const existingStyles = ['theme-style', 'background-style', 'transparency-style', 'font-style', 'theme-assets'];
  existingStyles.forEach(id => {
    const element = document.getElementById(id);
    if (element) element.remove();
  });
}

async function generateThemeCSS(theme) {
  const transparency = settings.transparency / 100;
  
  const cssVars = await generateCSSVariables(theme, transparency);
  const interactiveStyles = generateInteractiveStyles(theme);
  const clickThroughStyles = generateClickThroughStyles(theme);
  const commonStyles = generateCommonStyles(theme);
  const componentStyles = generateComponentStyles(theme);
  const customCSS = generateCustomCSS(theme);
  const backgroundStyles = await generateBackgroundStyles(theme);
  const layoutStyles = generateLayoutStyles(theme);
  const animationStyles = generateAnimationStyles(theme);
  const cssFileStyles = await loadCSSFile(theme);
  
  const finalCSS = `
    :root {
      ${cssVars}
    }
    
    ${backgroundStyles}
    ${layoutStyles}
    ${animationStyles}
    ${commonStyles}
    ${componentStyles}
    ${interactiveStyles}
    ${clickThroughStyles}
    ${customCSS}
    
    /* CSS File Styles */
    ${cssFileStyles}
  `;
  
  
  return finalCSS;
}

async function generateCSSVariables(theme, transparency) {
  const vars = [];
  
  vars.push(`--user-transparency: ${transparency}`);
  
  Object.entries(theme.colors).forEach(([category, values]) => {
    if (typeof values === 'object' && values !== null && !Array.isArray(values)) {
      Object.entries(values).forEach(([key, value]) => {
        const shortCategory = category === 'background' ? 'bg' : 
                             category === 'border' ? 'border' :
                             category === 'text' ? 'text' : category;
        
        // PG: Apply user transparency to background colors
        if (category === 'background' && value !== 'transparent') {
          // Parse rgba/rgb values and multiply alpha by user transparency
          const rgbaMatch = value.match(/rgba?\(([^)]+)\)/);
          if (rgbaMatch) {
            const parts = rgbaMatch[1].split(',').map(p => p.trim());
            if (parts.length === 4) {
              // Has alpha channel - multiply by user transparency
              const alpha = parseFloat(parts[3]);
              const newValue = `rgba(${parts[0]}, ${parts[1]}, ${parts[2]}, calc(${alpha} * var(--user-transparency, 1)))`;
              vars.push(`--${shortCategory}-${key}: ${newValue}`);
            } else if (parts.length === 3) {
              // No alpha channel - add user transparency
              const newValue = `rgba(${parts[0]}, ${parts[1]}, ${parts[2]}, var(--user-transparency, 1))`;
              vars.push(`--${shortCategory}-${key}: ${newValue}`);
            } else {
              vars.push(`--${shortCategory}-${key}: ${value}`);
            }
          } else {
            vars.push(`--${shortCategory}-${key}: ${value}`);
          }
        } else {
          vars.push(`--${shortCategory}-${key}: ${value}`);
        }
        
        if (category === 'border') {
          if (key === 'primary') vars.push(`--border-light: ${value}`);
          if (key === 'secondary') vars.push(`--border-dark: ${value}`);
        }
      });
    } else {
      vars.push(`--${category}: ${values}`);
    }
  });
  
  if (theme.effects) {
    Object.entries(theme.effects).forEach(([key, value]) => {
      const cssKey = key.replace(/([A-Z])/g, '-$1').toLowerCase();
      vars.push(`--${cssKey}: ${value}`);
    });
  }
  
  if (theme.transparency) {
    Object.entries(theme.transparency).forEach(([key, value]) => {
      vars.push(`--transparency-${key.replace(/([A-Z])/g, '-$1').toLowerCase()}: ${value}`);
    });
  }

  // Add asset variables
  if (theme.loadedAssets) {
    Object.entries(theme.loadedAssets).forEach(([assetName, assetUrl]) => {
      vars.push(`--asset-${assetName.replace(/[^a-zA-Z0-9]/g, '-')}: url('${assetUrl}')`);
    });
  }
  
  return vars.join(';\n      ');
}

async function loadThemeAssets(theme) {
  if (!theme.assets || Object.keys(theme.assets).length === 0) {
    theme.loadedAssets = {};
    return;
  }

  console.log('Loading theme assets for:', theme.name);
  theme.loadedAssets = {};

  for (const [assetName, assetInfo] of Object.entries(theme.assets)) {
    try {
      const assetData = await ipcRenderer.invoke('get-theme-asset', theme.id, assetName);
      if (assetData) {
        if (assetData.type === 'css') {
          // Store CSS content directly
          theme.loadedAssets[assetName] = assetData.data;
          console.log(`Loaded CSS: ${assetName}`);
        } else {
          // Create data URL for images
          const dataUrl = `data:${assetData.mimeType};base64,${assetData.data}`;
          theme.loadedAssets[assetName] = dataUrl;
          console.log(`Loaded asset: ${assetName} (${assetInfo.type})`);
        }
      }
    } catch (error) {
      console.error(`Failed to load asset ${assetName}:`, error);
    }
  }
}

async function generateBackgroundStyles(theme) {
  let backgroundCSS = '';

  // Handle full window background image - check for main_bg asset or backgroundImage property
  const mainBgAsset = theme.loadedAssets && theme.loadedAssets['main_bg'];
  const backgroundImageAsset = theme.backgroundImage && theme.loadedAssets && theme.loadedAssets[theme.backgroundImage.replace(/\.[^.]+$/, '')];
  const backgroundColor = theme.colors?.background?.primary;
  
  // Only apply background if we have an image or a non-transparent background color
  if (mainBgAsset || backgroundImageAsset || (backgroundColor && backgroundColor !== 'transparent')) {
    const assetUrl = mainBgAsset || backgroundImageAsset;
    
    backgroundCSS += `
      body {
        ${backgroundColor && backgroundColor !== 'transparent' ? `background-color: ${backgroundColor} !important;` : ''}
        ${assetUrl ? `
        background-image: url('${assetUrl}') !important;
        background-size: 100% 100% !important;
        background-position: center !important;
        background-repeat: no-repeat !important;
        background-attachment: fixed !important;` : ''}
      }
      
      .overlay-container.interactive {
        ${assetUrl ? 'background: transparent !important; background-color: transparent !important;' : ''}
      }
      .overlay-container.click-through {
        ${assetUrl ? 'background: transparent !important; background-color: transparent !important;' : ''}
      }
    `;
  }

  // Check for background configuration
  if (theme.backgrounds) {
    Object.entries(theme.backgrounds).forEach(([selector, bgConfig]) => {
      backgroundCSS += generateSelectorBackground(selector, bgConfig, theme);
    });
  }

  // Default backgrounds from assets
  if (theme.loadedAssets) {
    Object.entries(theme.loadedAssets).forEach(([assetName, assetUrl]) => {
      const assetInfo = theme.assets[assetName];
      if (assetInfo.type === 'background') {
        backgroundCSS += `
          .bg-${assetName} {
            background-image: url('${assetUrl}');
            background-size: cover;
            background-position: center;
            background-repeat: no-repeat;
          }
        `;
      }
    });
  }

  // Auto-generate CSS for common UI assets (can be overridden by CSS files)
  backgroundCSS += generateAutoAssetStyles(theme);

  return backgroundCSS;
}

function generateAutoAssetStyles(theme) {
  if (!theme.loadedAssets) return '';
  
  let autoCSS = '';
  
  // Define common asset to selector mappings
  const assetMappings = {
    'titlebar_bg': ['.overlay-container.interactive .header'],
    'button_normal': [
      '.overlay-container.interactive button',
      '.overlay-container.interactive .template-btn',
      '.overlay-container.interactive .add-btn',
      '.overlay-container.interactive .header-btn'
    ],
    'button_hover': [
      '.overlay-container.interactive button:hover',
      '.overlay-container.interactive .template-btn:hover', 
      '.overlay-container.interactive .add-btn:hover'
    ],
    'button_pressed': [
      '.overlay-container.interactive button:active',
      '.overlay-container.interactive .template-btn:active',
      '.overlay-container.interactive .add-btn:active'
    ],
    'progress_bg': ['.overlay-container.interactive .progress-bar'],
    'progress_fill': ['.overlay-container.interactive .progress-fill']
  };
  
  // Generate CSS for available assets
  Object.entries(assetMappings).forEach(([assetName, selectors]) => {
    const assetUrl = theme.loadedAssets[assetName];
    if (assetUrl) {
      selectors.forEach(selector => {
        autoCSS += `
          ${selector} {
            background-image: url('${assetUrl}') !important;
            background-size: cover !important;
            background-repeat: no-repeat !important;
          }
        `;
      });
    }
  });
  
  return autoCSS;
}

function generateSelectorBackground(selector, bgConfig, theme) {
  let bgCSS = `${selector} {\n`;

  if (bgConfig.asset && theme.loadedAssets[bgConfig.asset]) {
    bgCSS += `  background-image: url('${theme.loadedAssets[bgConfig.asset]}');\n`;
  }

  if (bgConfig.size) bgCSS += `  background-size: ${bgConfig.size};\n`;
  if (bgConfig.position) bgCSS += `  background-position: ${bgConfig.position};\n`;
  if (bgConfig.repeat) bgCSS += `  background-repeat: ${bgConfig.repeat};\n`;
  if (bgConfig.attachment) bgCSS += `  background-attachment: ${bgConfig.attachment};\n`;
  if (bgConfig.color) bgCSS += `  background-color: ${bgConfig.color};\n`;

  bgCSS += '}\n';
  return bgCSS;
}

function generateCSSFileStyles(theme) {
  let cssFileContent = '';

  if (theme.loadedAssets) {
    Object.entries(theme.loadedAssets).forEach(([assetName, assetContent]) => {
      const assetInfo = theme.assets[assetName];
      if (assetInfo && assetInfo.type === 'css') {
        cssFileContent += `\n/* CSS from ${assetName}.css */\n`;
        cssFileContent += assetContent;
        cssFileContent += '\n';
      }
    });
  }

  return cssFileContent;
}

function generateLayoutStyles(theme) {
  let layoutCSS = '';

  // Handle layout configuration
  if (theme.layout) {
    // Window sizing
    if (theme.layout.window) {
      const window = theme.layout.window;
      layoutCSS += `
        .overlay-container {
          ${window.width ? `width: ${window.width} !important;` : ''}
          ${window.height ? `height: ${window.height} !important;` : ''}
          ${window.minWidth ? `min-width: ${window.minWidth} !important;` : ''}
          ${window.minHeight ? `min-height: ${window.minHeight} !important;` : ''}
          ${window.maxWidth ? `max-width: ${window.maxWidth} !important;` : ''}
          ${window.maxHeight ? `max-height: ${window.maxHeight} !important;` : ''}
        }
      `;
    }

    // Component positioning
    if (theme.layout.components) {
      Object.entries(theme.layout.components).forEach(([component, layoutConfig]) => {
        const selectors = getLayoutSelectors(component);
        if (selectors.length > 0) {
          layoutCSS += `
            ${selectors.join(', ')} {
              ${generateLayoutProperties(layoutConfig)}
            }
          `;
        }
      });
    }

    // Custom positioning
    if (theme.layout.positions) {
      Object.entries(theme.layout.positions).forEach(([selector, posConfig]) => {
        layoutCSS += `
          ${selector} {
            ${generateLayoutProperties(posConfig)}
          }
        `;
      });
    }

    // Grid/flexbox layouts
    if (theme.layout.grid) {
      layoutCSS += generateGridStyles(theme.layout.grid);
    }

    if (theme.layout.flex) {
      layoutCSS += generateFlexStyles(theme.layout.flex);
    }
  }

  return layoutCSS;
}

function getLayoutSelectors(component) {
  const selectorMap = {
    'header': ['.header'],
    'templateSection': ['.template-section'],
    'tasksSection': ['.tasks-section'],
    'taskList': ['.tasks-list'],
    'taskItem': ['.task-item'],
    'progressBar': ['.progress-bar'],
    'buttons': ['button', '.template-btn', '.add-btn', '.header-btn'],
    'inputs': ['input', 'textarea', 'select']
  };
  
  return selectorMap[component] || [];
}

function generateLayoutProperties(layoutConfig) {
  const properties = [];
  
  // Position properties
  if (layoutConfig.position) properties.push(`position: ${layoutConfig.position} !important`);
  if (layoutConfig.top) properties.push(`top: ${layoutConfig.top} !important`);
  if (layoutConfig.right) properties.push(`right: ${layoutConfig.right} !important`);
  if (layoutConfig.bottom) properties.push(`bottom: ${layoutConfig.bottom} !important`);
  if (layoutConfig.left) properties.push(`left: ${layoutConfig.left} !important`);
  if (layoutConfig.zIndex) properties.push(`z-index: ${layoutConfig.zIndex} !important`);
  
  // Size properties
  if (layoutConfig.width) properties.push(`width: ${layoutConfig.width} !important`);
  if (layoutConfig.height) properties.push(`height: ${layoutConfig.height} !important`);
  if (layoutConfig.minWidth) properties.push(`min-width: ${layoutConfig.minWidth} !important`);
  if (layoutConfig.minHeight) properties.push(`min-height: ${layoutConfig.minHeight} !important`);
  if (layoutConfig.maxWidth) properties.push(`max-width: ${layoutConfig.maxWidth} !important`);
  if (layoutConfig.maxHeight) properties.push(`max-height: ${layoutConfig.maxHeight} !important`);
  
  // Spacing properties
  if (layoutConfig.margin) properties.push(`margin: ${layoutConfig.margin} !important`);
  if (layoutConfig.padding) properties.push(`padding: ${layoutConfig.padding} !important`);
  
  // Display properties
  if (layoutConfig.display) properties.push(`display: ${layoutConfig.display} !important`);
  if (layoutConfig.overflow) properties.push(`overflow: ${layoutConfig.overflow} !important`);
  if (layoutConfig.visibility) properties.push(`visibility: ${layoutConfig.visibility} !important`);
  
  // Flexbox properties
  if (layoutConfig.flex) properties.push(`flex: ${layoutConfig.flex} !important`);
  if (layoutConfig.flexDirection) properties.push(`flex-direction: ${layoutConfig.flexDirection} !important`);
  if (layoutConfig.justifyContent) properties.push(`justify-content: ${layoutConfig.justifyContent} !important`);
  if (layoutConfig.alignItems) properties.push(`align-items: ${layoutConfig.alignItems} !important`);
  if (layoutConfig.flexWrap) properties.push(`flex-wrap: ${layoutConfig.flexWrap} !important`);
  
  // Grid properties
  if (layoutConfig.gridColumn) properties.push(`grid-column: ${layoutConfig.gridColumn} !important`);
  if (layoutConfig.gridRow) properties.push(`grid-row: ${layoutConfig.gridRow} !important`);
  if (layoutConfig.gridArea) properties.push(`grid-area: ${layoutConfig.gridArea} !important`);
  
  return properties.join(';\n      ');
}

function generateGridStyles(gridConfig) {
  return `
    .layout-grid {
      display: grid !important;
      ${gridConfig.columns ? `grid-template-columns: ${gridConfig.columns} !important;` : ''}
      ${gridConfig.rows ? `grid-template-rows: ${gridConfig.rows} !important;` : ''}
      ${gridConfig.gap ? `gap: ${gridConfig.gap} !important;` : ''}
      ${gridConfig.areas ? `grid-template-areas: ${gridConfig.areas} !important;` : ''}
    }
  `;
}

function generateFlexStyles(flexConfig) {
  return `
    .layout-flex {
      display: flex !important;
      ${flexConfig.direction ? `flex-direction: ${flexConfig.direction} !important;` : ''}
      ${flexConfig.wrap ? `flex-wrap: ${flexConfig.wrap} !important;` : ''}
      ${flexConfig.justify ? `justify-content: ${flexConfig.justify} !important;` : ''}
      ${flexConfig.align ? `align-items: ${flexConfig.align} !important;` : ''}
      ${flexConfig.gap ? `gap: ${flexConfig.gap} !important;` : ''}
    }
  `;
}

function generateAnimationStyles(theme) {
  let animationCSS = '';

  // Handle animations configuration
  if (theme.animations) {
    // Generate keyframes
    if (theme.animations.keyframes) {
      Object.entries(theme.animations.keyframes).forEach(([name, keyframe]) => {
        animationCSS += `
          @keyframes ${name} {
            ${generateKeyframeCSS(keyframe)}
          }
        `;
      });
    }

    // Apply animations to components
    if (theme.animations.components) {
      Object.entries(theme.animations.components).forEach(([component, animConfig]) => {
        const selectors = getAnimationSelectors(component);
        if (selectors.length > 0) {
          animationCSS += `
            ${selectors.join(', ')} {
              ${generateAnimationProperties(animConfig)}
            }
          `;
        }
      });
    }

    // Custom animation selectors
    if (theme.animations.selectors) {
      Object.entries(theme.animations.selectors).forEach(([selector, animConfig]) => {
        animationCSS += `
          ${selector} {
            ${generateAnimationProperties(animConfig)}
          }
        `;
      });
    }

    // Transition effects
    if (theme.animations.transitions) {
      animationCSS += generateTransitionStyles(theme.animations.transitions);
    }
  }

  return animationCSS;
}

function generateKeyframeCSS(keyframe) {
  let keyframeCSS = '';
  
  Object.entries(keyframe).forEach(([percentage, styles]) => {
    keyframeCSS += `
      ${percentage} {
        ${objectToCSS(styles)}
      }
    `;
  });
  
  return keyframeCSS;
}

function getAnimationSelectors(component) {
  const selectorMap = {
    'container': ['.overlay-container'],
    'header': ['.header'],
    'tasks': ['.task-item'],
    'buttons': ['button', '.template-btn', '.add-btn'],
    'progress': ['.progress-bar', '.progress-fill'],
    'modal': ['.modal', '.settings-modal'],
    'inputs': ['input', 'textarea', 'select']
  };
  
  return selectorMap[component] || [];
}

function generateAnimationProperties(animConfig) {
  const properties = [];
  
  // Animation properties
  if (animConfig.name) properties.push(`animation-name: ${animConfig.name} !important`);
  if (animConfig.duration) properties.push(`animation-duration: ${animConfig.duration} !important`);
  if (animConfig.timing) properties.push(`animation-timing-function: ${animConfig.timing} !important`);
  if (animConfig.delay) properties.push(`animation-delay: ${animConfig.delay} !important`);
  if (animConfig.iteration) properties.push(`animation-iteration-count: ${animConfig.iteration} !important`);
  if (animConfig.direction) properties.push(`animation-direction: ${animConfig.direction} !important`);
  if (animConfig.fillMode) properties.push(`animation-fill-mode: ${animConfig.fillMode} !important`);
  if (animConfig.playState) properties.push(`animation-play-state: ${animConfig.playState} !important`);
  
  // Shorthand animation
  if (animConfig.animation) properties.push(`animation: ${animConfig.animation} !important`);
  
  // Transform properties
  if (animConfig.transform) properties.push(`transform: ${animConfig.transform} !important`);
  if (animConfig.transformOrigin) properties.push(`transform-origin: ${animConfig.transformOrigin} !important`);
  
  // Transition properties
  if (animConfig.transition) properties.push(`transition: ${animConfig.transition} !important`);
  if (animConfig.transitionProperty) properties.push(`transition-property: ${animConfig.transitionProperty} !important`);
  if (animConfig.transitionDuration) properties.push(`transition-duration: ${animConfig.transitionDuration} !important`);
  if (animConfig.transitionTiming) properties.push(`transition-timing-function: ${animConfig.transitionTiming} !important`);
  if (animConfig.transitionDelay) properties.push(`transition-delay: ${animConfig.transitionDelay} !important`);
  
  return properties.join(';\n      ');
}

function generateTransitionStyles(transitionConfig) {
  let transitionCSS = '';
  
  // Global transitions
  if (transitionConfig.global) {
    transitionCSS += `
      * {
        transition: ${transitionConfig.global} !important;
      }
    `;
  }
  
  // Component-specific transitions
  if (transitionConfig.components) {
    Object.entries(transitionConfig.components).forEach(([component, transition]) => {
      const selectors = getAnimationSelectors(component);
      if (selectors.length > 0) {
        transitionCSS += `
          ${selectors.join(', ')} {
            transition: ${transition} !important;
          }
        `;
      }
    });
  }
  
  // Hover effects
  if (transitionConfig.hover) {
    Object.entries(transitionConfig.hover).forEach(([selector, hoverConfig]) => {
      transitionCSS += `
        ${selector}:hover {
          ${generateAnimationProperties(hoverConfig)}
        }
      `;
    });
  }
  
  return transitionCSS;
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
  const globalEffects = generateGlobalEffects(theme);
  
  return `
    ${globalEffects}
    
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
      ${theme.effects?.borderRadius ? `border-radius: var(--border-radius) !important;` : ''}
    }
    
    .overlay-container .task-checkbox {
      ${theme.effects?.dropShadow ? `filter: var(--drop-shadow) !important;` : ''}
    }
    
    button, .template-btn, .add-btn, .header-btn, .modal-btn {
      ${theme.effects?.borderRadius ? `border-radius: var(--border-radius) !important;` : ''}
      ${theme.effects?.buttonShadow ? `box-shadow: var(--button-shadow) !important;` : ''}
      ${theme.effects?.textShadow ? `text-shadow: var(--text-shadow) !important;` : ''}
    }
    
    input, textarea, select, .task-input, .modal-input {
      ${theme.effects?.borderRadius ? `border-radius: var(--border-radius) !important;` : ''}
      ${theme.effects?.bevelInset ? `box-shadow: var(--bevel-inset) !important;` : ''}
    }
  `;
}

function generateGlobalEffects(theme) {
  if (!theme.effects) return '';
  
  let effectsCSS = '';
  
  // Handle bevel effects
  if (theme.effects.bevelInset || theme.effects.bevelOutset) {
    effectsCSS += `
      .bevel-inset {
        box-shadow: var(--bevel-inset) !important;
      }
      
      .bevel-outset {
        box-shadow: var(--bevel-outset) !important;
      }
    `;
  }
  
  // Handle text effects
  if (theme.effects.textShadow) {
    effectsCSS += `
      .text-shadow {
        text-shadow: var(--text-shadow) !important;
      }
    `;
  }
  
  // Handle drop shadow effects
  if (theme.effects.dropShadow) {
    effectsCSS += `
      .drop-shadow {
        filter: var(--drop-shadow) !important;
      }
    `;
  }
  
  // Handle glow effects
  if (theme.effects.glow) {
    effectsCSS += `
      .glow {
        box-shadow: var(--glow) !important;
      }
    `;
  }
  
  // Handle blur effects
  if (theme.effects.blur) {
    effectsCSS += `
      .blur {
        backdrop-filter: var(--blur) !important;
      }
    `;
  }
  
  return effectsCSS;
}

function generateComponentStyles(theme) {
  if (!theme.components) return '';
  
  let componentCSS = '';
  
  Object.entries(theme.components).forEach(([component, styles]) => {
    const selectors = getComponentSelectors(component);
    if (selectors.length > 0) {
      componentCSS += generateComponentCSS(component, selectors, styles, theme);
    }
  });
  
  return componentCSS;
}

function generateComponentCSS(component, selectors, styles, theme) {
  let css = '';
  
  // Handle different state styles (normal, hover, active, disabled)
  if (styles.states) {
    Object.entries(styles.states).forEach(([state, stateStyles]) => {
      const stateSelectors = selectors.map(selector => {
        switch(state) {
          case 'hover': return `${selector}:hover`;
          case 'active': return `${selector}:active`;
          case 'focus': return `${selector}:focus`;
          case 'disabled': return `${selector}:disabled`;
          default: return selector;
        }
      });
      
      css += `
        ${stateSelectors.join(', ')} {
          ${generateComponentProperties(stateStyles, theme)}
        }
      `;
    });
  } else {
    // Single state styling
    css += `
      ${selectors.join(', ')} {
        ${generateComponentProperties(styles, theme)}
      }
    `;
  }
  
  return css;
}

function generateComponentProperties(styles, theme) {
  let properties = [];
  
  Object.entries(styles).forEach(([key, value]) => {
    // Handle asset references
    if (typeof value === 'string' && value.startsWith('asset:')) {
      const assetName = value.substring(6);
      if (theme.loadedAssets && theme.loadedAssets[assetName]) {
        const cssKey = key.replace(/([A-Z])/g, '-$1').toLowerCase();
        
        if (key === 'backgroundImage' || key === 'background') {
          properties.push(`background-image: url('${theme.loadedAssets[assetName]}') !important`);
        } else if (key === 'borderImage') {
          properties.push(`border-image: url('${theme.loadedAssets[assetName]}') !important`);
        } else {
          properties.push(`${cssKey}: url('${theme.loadedAssets[assetName]}') !important`);
        }
      }
    } else {
      const cssKey = key.replace(/([A-Z])/g, '-$1').toLowerCase();
      properties.push(`${cssKey}: ${value} !important`);
    }
  });
  
  return properties.join(';\n      ');
}

function getComponentSelectors(component) {
  const selectorMap = {
    'button': [
      'button',
      '.template-btn',
      '.add-btn', 
      '.header-btn',
      '.modal-btn'
    ],
    'input': [
      'input[type="text"]',
      'input[type="password"]',
      'textarea',
      '.task-input',
      '.modal-input'
    ],
    'select': [
      'select',
      '.template-select'
    ],
    'progressBar': [
      '.progress-bar'
    ],
    'progressFill': [
      '.progress-fill'
    ],
    'taskItem': [
      '.task-item'
    ],
    'checkbox': [
      'input[type="checkbox"]',
      '.task-checkbox'
    ]
  };
  
  return selectorMap[component] || [];
}

function generateCustomCSS(theme) {
  if (!theme.customCSS) return '';
  
  let customCSS = '';
  
  if (typeof theme.customCSS === 'string') {
    customCSS = theme.customCSS;
  } else if (typeof theme.customCSS === 'object') {
    Object.entries(theme.customCSS).forEach(([key, styles]) => {
      if (typeof styles === 'string') {
        customCSS += styles + '\n';
      } else if (typeof styles === 'object') {
        customCSS += `
          .theme-${key} {
            ${objectToCSS(styles)}
          }
        `;
      }
    });
  }
  
  return customCSS;
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

async function loadCSSFile(theme) {
  if (!theme.cssFile) return '';
  
  try {
    const cssContent = await ipcRenderer.invoke('load-theme-css', theme.id, theme.cssFile);
    return cssContent || '';
  } catch (error) {
    console.warn(`Failed to load CSS file ${theme.cssFile}:`, error);
    return '';
  }
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
    if (window.recordingHotkey) {
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
        completeNextTask: 'Ctrl+Shift+N',
        undoLastAction: 'Ctrl+Shift+Z'
      };
    }

    // Get current hotkey values from input fields (in case recording didn't update them)
    const visibilityInput = document.getElementById('toggleVisibilityHotkey');
    const interactiveInput = document.getElementById('toggleInteractiveHotkey');
    const completeTaskInput = document.getElementById('completeNextTaskHotkey');
    const undoActionInput = document.getElementById('undoLastActionHotkey');

    if (visibilityInput.value.trim()) {
      settings.hotkeys.toggleVisibility = visibilityInput.value.trim();
    }
    if (interactiveInput.value.trim()) {
      settings.hotkeys.toggleInteractive = interactiveInput.value.trim();
    }
    if (completeTaskInput.value.trim()) {
      settings.hotkeys.completeNextTask = completeTaskInput.value.trim();
    }
    if (undoActionInput.value.trim()) {
      settings.hotkeys.undoLastAction = undoActionInput.value.trim();
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
      Object.assign(settings, loadedSettings, {
        hotkeys: {
          ...settings.hotkeys,
          ...(loadedSettings.hotkeys || {})
        }
      });
      
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
  } else if (hotkeyType === 'undoLastAction') {
    button = document.getElementById('recordBtn4');
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
    } else if (window.recordingHotkey === 'undoLastAction') {
      button = document.getElementById('recordBtn4');
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
  const wasInteractive = state.isInteractiveMode;
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


async function loadThemes() {
  try {
    replaceArray(themes, await ipcRenderer.invoke('load-themes'));
    console.log('Themes loaded:', themes.length);
  } catch (error) {
    console.error('Failed to load themes:', error);
    replaceArray(themes, []);
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
    replaceArray(themes, await ipcRenderer.invoke('reload-themes'));
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


  async function saveSettingsData() {
    try {
      await ipcRenderer.invoke('save-settings', settings);
    } catch (error) {
      console.error('Failed to save settings:', error);
    }
  }

  return {
    resetHotkeys,
    showSettingsModal,
    updateThemeSelector,
    closeSettingsModal,
    updateTransparencyControls,
    updateTransparency,
    updateTheme,
    applyTheme,
    removeExistingStyles,
    generateThemeCSS,
    generateCSSVariables,
    loadThemeAssets,
    generateBackgroundStyles,
    generateAutoAssetStyles,
    generateSelectorBackground,
    generateCSSFileStyles,
    generateLayoutStyles,
    getLayoutSelectors,
    generateLayoutProperties,
    generateGridStyles,
    generateFlexStyles,
    generateAnimationStyles,
    generateKeyframeCSS,
    getAnimationSelectors,
    generateAnimationProperties,
    generateTransitionStyles,
    generateInteractiveStyles,
    generateClickThroughStyles,
    generateCommonStyles,
    generateGlobalEffects,
    generateComponentStyles,
    generateComponentCSS,
    generateComponentProperties,
    getComponentSelectors,
    generateCustomCSS,
    objectToCSS,
    getCurrentTheme,
    loadCSSFile,
    applyFonts,
    applyTransparencySettings,
    saveSettings,
    loadSettings,
    saveSettingsData,
    resetPosition,
    recordHotkey,
    stopRecording,
    handleHotkeyRecording,
    previewBackground,
    loadThemes,
    openThemesFolder,
    reloadThemes,
    showThemesPath
  };
}
