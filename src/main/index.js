// Main Electron process for Task Overlay
const { app, BrowserWindow, screen, ipcMain, Menu, globalShortcut, shell } = require('electron');
const path = require('path');
const fs = require('fs').promises;
const { exec } = require('child_process');

let overlayWindow = null;
let isGameRunning = false;
let gameMonitorInterval = null;
let manuallyHidden = false;
let isInteractive = true; // Start interactive so inputs work
let currentHotkeys = {
  toggleVisibility: 'CommandOrControl+Shift+T',
  toggleInteractive: 'CommandOrControl+Shift+I',
  completeNextTask: 'CommandOrControl+Shift+N'
};

// Data storage paths - Use proper user data directory
const DATA_DIR = app.isPackaged 
  ? path.join(app.getPath('userData'), 'data')
  : path.join(__dirname, 'data');

const THEMES_DIR = path.join(DATA_DIR, 'themes');
const TASKS_FILE = path.join(DATA_DIR, 'current_tasks.json');
const TEMPLATES_FILE = path.join(DATA_DIR, 'templates.json');
const SETTINGS_FILE = path.join(DATA_DIR, 'settings.json');

class PoE2TaskOverlay {
  constructor() {
    this.themes = new Map();
    this.initializeDataDirectory();
  }

  async initializeDataDirectory() {
    try {
      console.log('Creating data directory at:', DATA_DIR);
      await fs.mkdir(DATA_DIR, { recursive: true });
      await fs.mkdir(THEMES_DIR, { recursive: true });
      console.log('Data directory created successfully');
      await this.copyDefaultThemes();
      await this.loadThemes();
    } catch (error) {
      console.error('Failed to create data directory:', error);
    }
  }

  async copyDefaultThemes() {
    try {
      const sourceThemesDir = path.join(__dirname, '../../data/themes');
      console.log('Copying default themes from:', sourceThemesDir);
      
      const themeFiles = await fs.readdir(sourceThemesDir);
      
      for (const file of themeFiles) {
        if (!file.endsWith('.json')) continue;
        
        const sourcePath = path.join(sourceThemesDir, file);
        const targetPath = path.join(THEMES_DIR, file);
        
        try {
          await fs.access(targetPath);
          console.log(`Theme already exists: ${file}`);
        } catch {
          console.log(`Copying default theme: ${file}`);
          await fs.copyFile(sourcePath, targetPath);
        }
      }
    } catch (error) {
      console.error('Failed to copy default themes:', error);
    }
  }

  async loadThemes() {
    try {
      console.log('Loading themes from:', THEMES_DIR);
      const themeFiles = await fs.readdir(THEMES_DIR);
      
      for (const file of themeFiles) {
        if (!file.endsWith('.json')) continue;
        
        try {
          const themePath = path.join(THEMES_DIR, file);
          const themeData = await fs.readFile(themePath, 'utf8');
          const theme = JSON.parse(themeData);
          
          if (theme.id && theme.name) {
            this.themes.set(theme.id, theme);
            console.log(`Loaded theme: ${theme.name} (${theme.id})`);
          } else {
            console.warn(`Invalid theme file: ${file} - missing id or name`);
          }
        } catch (error) {
          console.error(`Failed to load theme file ${file}:`, error);
        }
      }
      
      console.log(`Total themes loaded: ${this.themes.size}`);
    } catch (error) {
      console.error('Failed to load themes:', error);
    }
  }

  async initialize() {
    await app.whenReady();
    await this.loadSettingsOnStartup();
    this.createOverlayWindow();
    this.startGameMonitoring();
    this.setupIPC();
    this.setupMenu();
  }

  async loadSettingsOnStartup() {
    try {
      console.log('Loading settings on startup...');
      const data = await fs.readFile(SETTINGS_FILE, 'utf8');
      const settings = JSON.parse(data);
      console.log('Settings loaded on startup:', settings);
      
      if (settings.hotkeys) {
        console.log('Updating hotkeys from settings:', settings.hotkeys);
        currentHotkeys.toggleVisibility = this.convertHotkeyFormat(settings.hotkeys.toggleVisibility);
        currentHotkeys.toggleInteractive = this.convertHotkeyFormat(settings.hotkeys.toggleInteractive);
        currentHotkeys.completeNextTask = this.convertHotkeyFormat(settings.hotkeys.completeNextTask);
        console.log('Updated currentHotkeys:', currentHotkeys);
      }
    } catch (error) {
      console.log('No settings file found on startup, using defaults:', error.message);
    }
  }

  createOverlayWindow() {
    const primaryDisplay = screen.getPrimaryDisplay();
    const { width, height } = primaryDisplay.workAreaSize;

    overlayWindow = new BrowserWindow({
      width: 500,
      height: 700,
      minWidth: 450,
      minHeight: 600,
      x: width - 520,
      y: 20,
      transparent: true,
      frame: false,
      alwaysOnTop: true,
      skipTaskbar: true,
      focusable: true, // Always allow focus for inputs to work
      resizable: true,
      webSecurity: false,
      minimizable: false,
      maximizable: false,
      show: false, // Start hidden
      webPreferences: {
        nodeIntegration: true,
        contextIsolation: false,
        webSecurity: false,
        enableRemoteModule: true,
        allowRunningInsecureContent: true
      }
    });

    // Force always on top more aggressively
    overlayWindow.setAlwaysOnTop(true, 'screen-saver');
    overlayWindow.setVisibleOnAllWorkspaces(true);

    // Load the renderer function
    const loadRenderer = () => {
      if (!app.isPackaged && process.env.NODE_ENV === 'development' && process.env['ELECTRON_RENDERER_URL']) {
        console.log('Loading dev URL:', process.env['ELECTRON_RENDERER_URL']);
        overlayWindow.loadURL(process.env['ELECTRON_RENDERER_URL']);
      } else {
        const rendererPath = path.join(__dirname, '../renderer/index.html');
        console.log('Loading file:', rendererPath);
        console.log('File exists:', require('fs').existsSync(rendererPath));
        overlayWindow.loadFile(rendererPath);
      }
    };

    // Add error handling for renderer loading
    overlayWindow.webContents.on('did-fail-load', (event, errorCode, errorDescription, validatedURL) => {
      console.error('Failed to load renderer:', errorCode, errorDescription, validatedURL);
      
      // Try alternative path
      const altPath = path.join(__dirname, '../../dist-electron/renderer/index.html');
      console.log('Trying alternative path:', altPath);
      if (require('fs').existsSync(altPath)) {
        overlayWindow.loadFile(altPath);
      }
    });

    overlayWindow.webContents.on('did-finish-load', () => {
      console.log('Renderer loaded successfully');
      
      // Check if JavaScript is working
      overlayWindow.webContents.executeJavaScript('typeof window !== "undefined"')
        .then(result => {
          console.log('JavaScript working:', result);
          
          // Test if our functions exist
          overlayWindow.webContents.executeJavaScript('typeof addTask !== "undefined"')
            .then(hasFunction => {
              console.log('addTask function exists:', hasFunction);
              if (!hasFunction) {
                console.error('Renderer script not loaded properly');
              }
            });
        })
        .catch(error => {
          console.error('JavaScript execution failed:', error);
        });

      // Register hotkeys after window is fully loaded
      console.log('Registering hotkeys after window load...');
      setTimeout(() => {
        this.registerHotkeys();
        
        // Verify registration
        setTimeout(() => {
          const isRegistered = globalShortcut.isRegistered(currentHotkeys.toggleVisibility);
          console.log(`Hotkey registration check: ${isRegistered ? 'SUCCESS' : 'FAILED'}`);
        }, 1000);
      }, app.isPackaged ? 2000 : 500);
    });

    // !!! CRITICAL !!! Actually call the loadRenderer function!
    loadRenderer();
    
    // Start in interactive mode
    overlayWindow.setIgnoreMouseEvents(false);
    isInteractive = true;
    
    // Event handlers to maintain overlay properties
    overlayWindow.on('blur', () => {
      // Maintain always on top when losing focus
      if (overlayWindow && !overlayWindow.isDestroyed()) {
        overlayWindow.setAlwaysOnTop(true, 'screen-saver');
      }
    });
    
    overlayWindow.on('focus', () => {
      // Ensure proper overlay state when gaining focus
      if (overlayWindow && !overlayWindow.isDestroyed()) {
        overlayWindow.setAlwaysOnTop(true, 'screen-saver');
      }
    });

    // Show window after a delay to ensure everything loads
    setTimeout(() => {
      overlayWindow.show();
    }, 1000);
    
    // Optional: Show dev tools in development
    // overlayWindow.webContents.openDevTools();
  }

  setupMenu() {
    // Hotkeys are now registered after window loads, not here

    const template = [
      {
        label: 'File',
        submenu: [
          {
            label: 'Show/Hide Overlay',
            accelerator: currentHotkeys.toggleVisibility.replace('CommandOrControl', 'Ctrl'),
            click: () => {
              console.log('Menu shortcut clicked');
              if (overlayWindow.isVisible()) {
                overlayWindow.hide();
                manuallyHidden = true;
              } else {
                overlayWindow.show();
                overlayWindow.setAlwaysOnTop(true, 'screen-saver');
                overlayWindow.focus();
                manuallyHidden = false;
              }
            }
          },
          {
            label: 'Toggle Interactive Mode',
            accelerator: currentHotkeys.toggleInteractive.replace('CommandOrControl', 'Ctrl'),
            click: () => {
              if (overlayWindow && overlayWindow.isVisible()) {
                this.toggleInteractiveMode();
              }
            }
          },
          {
            label: 'Complete Next Task',
            accelerator: currentHotkeys.completeNextTask.replace('CommandOrControl', 'Ctrl'),
            click: () => {
              if (overlayWindow && overlayWindow.isVisible()) {
                overlayWindow.webContents.send('complete-next-task');
              }
            }
          },
          { type: 'separator' },
          { role: 'quit' }
        ]
      }
    ];
    
    const menu = Menu.buildFromTemplate(template);
    Menu.setApplicationMenu(menu);
  }

  registerHotkeys() {
    try {
      console.log('=== REGISTERING HOTKEYS ===');
      console.log('App packaged:', app.isPackaged);
      console.log('Current hotkeys:', currentHotkeys);
      
      // Unregister any existing shortcuts
      globalShortcut.unregisterAll();
      
      // Check if globalShortcut is available
      if (!globalShortcut) {
        console.error('globalShortcut not available');
        return;
      }
      
      // Toggle overlay visibility
      const toggleRet = globalShortcut.register(currentHotkeys.toggleVisibility, () => {
        console.log('Toggle visibility shortcut triggered!');
        if (overlayWindow) {
          if (overlayWindow.isVisible()) {
            console.log('Hiding overlay');
            overlayWindow.hide();
            manuallyHidden = true;
          } else {
            console.log('Showing overlay');
            overlayWindow.show();
            overlayWindow.setAlwaysOnTop(true, 'screen-saver');
            overlayWindow.focus();
            manuallyHidden = false;
          }
        }
      });

      // Toggle interactive mode
      const interactiveRet = globalShortcut.register(currentHotkeys.toggleInteractive, () => {
        console.log('Toggle interactive mode triggered!');
        if (overlayWindow && overlayWindow.isVisible()) {
          this.toggleInteractiveMode();
        }
      });

      // Complete next task
      const nextTaskRet = globalShortcut.register(currentHotkeys.completeNextTask, () => {
        console.log('Complete next task shortcut triggered!');
        if (overlayWindow && overlayWindow.isVisible()) {
          overlayWindow.webContents.send('complete-next-task');
        }
      });

      // Log results with better formatting
      console.log('Hotkey Registration Results:');
      console.log(`  ${currentHotkeys.toggleVisibility}: ${toggleRet ? 'SUCCESS' : 'FAILED'}`);
      console.log(`  ${currentHotkeys.toggleInteractive}: ${interactiveRet ? 'SUCCESS' : 'FAILED'}`);
      console.log(`  ${currentHotkeys.completeNextTask}: ${nextTaskRet ? 'SUCCESS' : 'FAILED'}`);

      const successCount = [toggleRet, interactiveRet, nextTaskRet].filter(Boolean).length;
      console.log(`Total: ${successCount}/3 hotkeys registered`);

      if (successCount === 0) {
        console.error('No hotkeys registered successfully');
        throw new Error('Hotkey registration completely failed');
      }

    } catch (error) {
      console.error('Error in registerHotkeys:', error);
      // Don't throw, let the fallback system handle it
    }
  }

  toggleInteractiveMode() {
    isInteractive = !isInteractive;
    console.log('Toggling interactive mode to:', isInteractive ? 'ON' : 'OFF');
    
    // Update mouse events without recreating window
    overlayWindow.setIgnoreMouseEvents(!isInteractive);
    
    // Send state to renderer for visual feedback
    overlayWindow.webContents.send('interactive-mode-changed', isInteractive);
  }

  startGameMonitoring() {
    gameMonitorInterval = setInterval(() => {
      this.checkForPoE2();
    }, 3000);
  }

  checkForPoE2() {
    exec('tasklist /fo csv | findstr /i "PathOfExile"', (error, stdout) => {
      const wasRunning = isGameRunning;
      isGameRunning = stdout.includes('PathOfExile');
      
      if (isGameRunning && !wasRunning && !manuallyHidden) {
        console.log('PoE2 detected - showing overlay in interactive mode');
        isInteractive = true;
        overlayWindow.show();
        overlayWindow.setAlwaysOnTop(true, 'screen-saver');
        overlayWindow.setIgnoreMouseEvents(false);
        overlayWindow.webContents.send('interactive-mode-changed', true);
      } else if (!isGameRunning && wasRunning) {
        console.log('PoE2 closed - hiding overlay');
        overlayWindow.hide();
        manuallyHidden = false;
        isInteractive = true;
      }
      
      // Ensure overlay stays on top if game is running and overlay is visible
      if (isGameRunning && overlayWindow && overlayWindow.isVisible()) {
        overlayWindow.setAlwaysOnTop(true, 'screen-saver');
      }
    });
  }

  setupIPC() {
    // Load current tasks
    ipcMain.handle('load-tasks', async () => {
      try {
        const data = await fs.readFile(TASKS_FILE, 'utf8');
        return JSON.parse(data);
      } catch (error) {
        return { tasks: [], currentTemplate: null };
      }
    });

    // Save current tasks
    ipcMain.handle('save-tasks', async (event, tasksData) => {
      try {
        await fs.writeFile(TASKS_FILE, JSON.stringify(tasksData, null, 2));
        return { success: true };
      } catch (error) {
        console.error('Failed to save tasks:', error);
        return { success: false, error: error.message };
      }
    });

    // Load templates
    ipcMain.handle('load-templates', async () => {
      try {
        const data = await fs.readFile(TEMPLATES_FILE, 'utf8');
        return JSON.parse(data);
      } catch (error) {
        return [];
      }
    });

    // Save templates
    ipcMain.handle('save-templates', async (event, templates) => {
      try {
        await fs.writeFile(TEMPLATES_FILE, JSON.stringify(templates, null, 2));
        return { success: true };
      } catch (error) {
        console.error('Failed to save templates:', error);
        return { success: false, error: error.message };
      }
    });

    // Save settings
    ipcMain.handle('save-settings', async (event, settings) => {
      try {
        console.log('Attempting to save settings:', settings);
        console.log('Settings file path:', SETTINGS_FILE);
        
        await fs.mkdir(DATA_DIR, { recursive: true });
        
        const settingsJson = JSON.stringify(settings, null, 2);
        console.log('Settings JSON:', settingsJson);
        
        await fs.writeFile(SETTINGS_FILE, settingsJson);
        console.log('Settings saved successfully to:', SETTINGS_FILE);
        
        return { success: true };
      } catch (error) {
        console.error('Failed to save settings:', error);
        return { success: false, error: error.message };
      }
    });

    // Load settings
    ipcMain.handle('load-settings', async () => {
      try {
        console.log('Attempting to load settings from:', SETTINGS_FILE);
        const data = await fs.readFile(SETTINGS_FILE, 'utf8');
        const settings = JSON.parse(data);
        console.log('Settings loaded successfully:', settings);
        return settings;
      } catch (error) {
        console.log('Settings file not found or invalid, using defaults:', error.message);
        return null;
      }
    });

    // Load themes
    ipcMain.handle('load-themes', async () => {
      try {
        return Array.from(this.themes.values());
      } catch (error) {
        console.error('Failed to get themes:', error);
        return [];
      }
    });

    // Get specific theme
    ipcMain.handle('get-theme', async (event, themeId) => {
      try {
        return this.themes.get(themeId) || null;
      } catch (error) {
        console.error('Failed to get theme:', error);
        return null;
      }
    });

    // Reload themes
    ipcMain.handle('reload-themes', async () => {
      try {
        this.themes.clear();
        await this.loadThemes();
        return Array.from(this.themes.values());
      } catch (error) {
        console.error('Failed to reload themes:', error);
        return [];
      }
    });

    // Open themes folder
    ipcMain.handle('open-themes-folder', async () => {
      try {
        await shell.openPath(THEMES_DIR);
        return { success: true };
      } catch (error) {
        console.error('Failed to open themes folder:', error);
        return { success: false, error: error.message };
      }
    });

    // Get themes folder path
    ipcMain.handle('get-themes-path', async () => {
      return THEMES_DIR;
    });

    // Update hotkeys
    ipcMain.on('update-hotkeys', (event, hotkeys) => {
      console.log('Received hotkey update request:', hotkeys);
      
      const newToggleVisibility = this.convertHotkeyFormat(hotkeys.toggleVisibility);
      const newToggleInteractive = this.convertHotkeyFormat(hotkeys.toggleInteractive);
      const newCompleteNextTask = this.convertHotkeyFormat(hotkeys.completeNextTask);
      
      console.log('Converted hotkeys:', {
        visibility: newToggleVisibility,
        interactive: newToggleInteractive,
        completeNextTask: newCompleteNextTask
      });
      
      if (newToggleVisibility !== currentHotkeys.toggleVisibility || 
          newToggleInteractive !== currentHotkeys.toggleInteractive ||
          newCompleteNextTask !== currentHotkeys.completeNextTask) {
        
        currentHotkeys.toggleVisibility = newToggleVisibility;
        currentHotkeys.toggleInteractive = newToggleInteractive;
        currentHotkeys.completeNextTask = newCompleteNextTask;
        
        this.registerHotkeys();
      }
    });

    // Reset window position
    ipcMain.on('reset-window-position', () => {
      if (overlayWindow) {
        const primaryDisplay = screen.getPrimaryDisplay();
        const { width } = primaryDisplay.workAreaSize;
        overlayWindow.setPosition(width - 520, 20);
      }
    });

    // Handle window movement
    ipcMain.on('move-window', (event, { x, y }) => {
      if (overlayWindow) {
        overlayWindow.setPosition(x, y);
      }
    });

    // Manual show/hide for testing
    ipcMain.on('toggle-overlay', () => {
      console.log('IPC: toggle-overlay received');
      if (overlayWindow) {
        if (overlayWindow.isVisible()) {
          console.log('IPC: Hiding overlay');
          overlayWindow.hide();
          manuallyHidden = true;
        } else {
          console.log('IPC: Showing overlay');
          overlayWindow.show();
          overlayWindow.setAlwaysOnTop(true, 'screen-saver');
          overlayWindow.focus();
          manuallyHidden = false;
        }
      }
    });

    // Minimize overlay
    ipcMain.on('minimize-overlay', () => {
      console.log('IPC: minimize-overlay received');
      if (overlayWindow) {
        console.log('IPC: Hiding overlay');
        overlayWindow.hide();
        manuallyHidden = true;
      }
    });

    // Show overlay
    ipcMain.on('show-overlay', () => {
      console.log('IPC: show-overlay received');
      if (overlayWindow) {
        overlayWindow.show();
        overlayWindow.setAlwaysOnTop(true, 'screen-saver');
        overlayWindow.focus();
        manuallyHidden = false;
      }
    });

    // Toggle interactive mode - DON'T recreate window
    ipcMain.on('toggle-interactive-mode', () => {
      console.log('IPC: toggle-interactive-mode received');
      if (overlayWindow && overlayWindow.isVisible()) {
        this.toggleInteractiveMode();
      }
    });

    // Quit application
    ipcMain.on('quit-application', () => {
      console.log('IPC: quit-application received');
      this.cleanup();
      app.quit();
    });
  }

  convertHotkeyFormat(userHotkey) {
    if (!userHotkey) return 'CommandOrControl+Shift+T';
    
    console.log('Converting hotkey:', userHotkey);
    
    if (userHotkey.includes('CommandOrControl')) {
      console.log('Already in Electron format:', userHotkey);
      return userHotkey;
    }
    
    let converted = userHotkey
      .replace(/\bCtrl\b/gi, 'CommandOrControl')
      .replace(/\bCmd\b/gi, 'CommandOrControl')
      .replace(/\bCommand\b/gi, 'CommandOrControl');
    
    console.log('Converted to:', converted);
    return converted;
  }

  cleanup() {
    if (gameMonitorInterval) {
      clearInterval(gameMonitorInterval);
    }
    globalShortcut.unregisterAll();
  }
}

// Initialize overlay
const overlay = new PoE2TaskOverlay();

app.whenReady().then(() => {
  overlay.initialize();
});

app.on('window-all-closed', () => {
  overlay.cleanup();
  if (process.platform !== 'darwin') {
    app.quit();
  }
});

app.on('activate', () => {
  if (BrowserWindow.getAllWindows().length === 0) {
    overlay.createOverlayWindow();
  }
});

app.on('will-quit', () => {
  globalShortcut.unregisterAll();
});