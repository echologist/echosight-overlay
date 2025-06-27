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
    
    // Load theme assets first
    await loadThemeAssets(theme);
    
    const style = document.createElement('style');
    style.id = 'theme-style';
    
    let css = await generateThemeCSS(theme);
    console.log('Generated CSS:', css);
    style.textContent = css;
    document.head.appendChild(style);
    
    applyFonts(theme);
    
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
        vars.push(`--${shortCategory}-${key}: ${value}`);
        
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
function completeNextTask() {
  try {
    const nextTask = tasks.find(t => !t.completed);
    if (nextTask) {
      nextTask.completed = true;
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
      console.log('No uncompleted tasks found');
    }
  } catch (error) {
    console.error('Error completing next task:', error);
  }
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
function addTask() {
  try {
    console.log('addTask called');
    const input = document.getElementById('taskInput');
    const text = input.value.trim();
    console.log('Task text:', text);

    if (text) {
      const task = {
        id: Date.now(),
        text: text,
        completed: false,
        createdAt: new Date().toISOString()
      };

      tasks.push(task);
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

function toggleTask(taskId) {
  const task = tasks.find(t => t.id === taskId);
  if (task) {
    task.completed = !task.completed;
    renderTasks();
    updateProgress();
    saveTasks();
  }
}

function deleteTask(taskId) {
  tasks = tasks.filter(t => t.id !== taskId);
  renderTasks();
  updateProgress();
  saveTasks();
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

// Rendering
function renderTasks() {
  const taskList = document.getElementById('taskList');
  taskList.innerHTML = '';

  tasks.forEach(task => {
    const li = document.createElement('li');
    li.className = 'task-item';
    li.dataset.taskId = task.id;  // Sets the task ID for drag and drop
    li.draggable = true;  // Allows dragging of this task item
    
    li.innerHTML = `
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
      <input type="checkbox" class="task-checkbox" ${task.completed ? 'checked' : ''} 
             onchange="toggleTask(${task.id})">
      <span class="task-text ${task.completed ? 'completed' : ''}">${escapeHtml(task.text)}</span>
      <button class="task-delete" onclick="deleteTask(${task.id})">×</button>
    `;
    taskList.appendChild(li);
  });
  
  // Initialize drag and drop for the new elements
  initializeDragAndDrop();
}

function updateProgress() {
  const completed = tasks.filter(t => t.completed).length;
  const total = tasks.length;
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
window.clearAllTasks = clearAllTasks;
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