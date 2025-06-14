// // main.js - Main Electron process for PoE2 Task Overlay
// const { app, BrowserWindow, screen, ipcMain, Menu, globalShortcut } = require('electron');
// const path = require('path');
// const fs = require('fs').promises;
// const { exec } = require('child_process');

// let overlayWindow = null;
// let isGameRunning = false;
// let gameMonitorInterval = null;
// let manuallyHidden = false; // Track if user manually hid the overlay
// let isInteractive = false; // Track if overlay is interactive or click-through
// let currentHotkeys = {
//   toggleVisibility: 'CommandOrControl+Shift+T',
//   toggleInteractive: 'CommandOrControl+Shift+I'
// };

// // Data storage paths
// const DATA_DIR = app.isPackaged 
//   ? path.join(app.getPath('userData'), 'data')
//   : path.join(__dirname, 'data');

// const TASKS_FILE = path.join(DATA_DIR, 'current_tasks.json');
// const TEMPLATES_FILE = path.join(DATA_DIR, 'templates.json');
// const SETTINGS_FILE = path.join(DATA_DIR, 'settings.json');

// class PoE2TaskOverlay {
//   constructor() {
//     this.initializeDataDirectory();
//   }

// async initializeDataDirectory() {
//   try {
//     console.log('Creating data directory at:', DATA_DIR);
//     await fs.mkdir(DATA_DIR, { recursive: true });
//     console.log('Data directory created successfully');
//   } catch (error) {
//     console.error('Failed to create data directory:', error);
//   }
// }

//   async initialize() {
//     await app.whenReady();
//     await this.loadSettingsOnStartup(); // Load settings before creating window
//     this.createOverlayWindow();
//     this.startGameMonitoring();
//     this.setupIPC();
//     this.setupMenu();
//   }

//   async loadSettingsOnStartup() {
//     try {
//       console.log('Loading settings on startup...');
//       const data = await fs.readFile(SETTINGS_FILE, 'utf8');
//       const settings = JSON.parse(data);
//       console.log('Settings loaded on startup:', settings);
      
//       // Update hotkeys if they exist in settings
//       if (settings.hotkeys) {
//         console.log('Updating hotkeys from settings:', settings.hotkeys);
//         currentHotkeys.toggleVisibility = this.convertHotkeyFormat(settings.hotkeys.toggleVisibility);
//         currentHotkeys.toggleInteractive = this.convertHotkeyFormat(settings.hotkeys.toggleInteractive);
//         console.log('Updated currentHotkeys:', currentHotkeys);
//       }
//     } catch (error) {
//       console.log('No settings file found on startup, using defaults:', error.message);
//     }
//   }

//   createOverlayWindow() {
//     const primaryDisplay = screen.getPrimaryDisplay();
//     const { width, height } = primaryDisplay.workAreaSize;

//     overlayWindow = new BrowserWindow({
//       width: 500,        // Much larger width
//       height: 700,       // Much larger height  
//       minWidth: 450,     // Larger minimum width
//       minHeight: 600,    // Larger minimum height
//       x: width - 520,    // Adjusted for new width
//       y: 20,
//       transparent: true,
//       frame: false,
//       alwaysOnTop: true,
//       skipTaskbar: true,
//       resizable: true,   // Make sure this is true
//       webSecurity: false,
//       minimizable: false,
//       maximizable: false,
//       webPreferences: {
//         nodeIntegration: true,
//         contextIsolation: false
//       }
//     });

//     // Force always on top more aggressively
//     overlayWindow.setAlwaysOnTop(true, 'screen-saver');
//     overlayWindow.setVisibleOnAllWorkspaces(true);

//     overlayWindow.loadFile('renderer/index.html');
    
//     // Start in click-through mode (doesn't interfere with game)
//     overlayWindow.setIgnoreMouseEvents(true);
//     overlayWindow.hide(); // Hide initially until game is detected
    
//     // Optional: Show dev tools in development
//     // overlayWindow.webContents.openDevTools();
//   }

//   setupMenu() {
//     this.registerHotkeys();

//     const template = [
//       {
//         label: 'File',
//         submenu: [
//           {
//             label: 'Show/Hide Overlay',
//             accelerator: currentHotkeys.toggleVisibility.replace('CommandOrControl', 'Ctrl'),
//             click: () => {
//               console.log('Menu shortcut clicked');
//               if (overlayWindow.isVisible()) {
//                 overlayWindow.hide();
//                 manuallyHidden = true;
//               } else {
//                 overlayWindow.show();
//                 overlayWindow.setAlwaysOnTop(true, 'screen-saver');
//                 overlayWindow.focus();
//                 manuallyHidden = false;
//               }
//             }
//           },
//           {
//             label: 'Toggle Interactive Mode',
//             accelerator: currentHotkeys.toggleInteractive.replace('CommandOrControl', 'Ctrl'),
//             click: () => {
//               if (overlayWindow && overlayWindow.isVisible()) {
//                 isInteractive = !isInteractive;
//                 overlayWindow.setIgnoreMouseEvents(!isInteractive);
//                 overlayWindow.webContents.send('interactive-mode-changed', isInteractive);
//                 console.log('Interactive mode:', isInteractive ? 'ON' : 'OFF');
//               }
//             }
//           },
//           { type: 'separator' },
//           { role: 'quit' }
//         ]
//       }
//     ];
    
//     const menu = Menu.buildFromTemplate(template);
//     Menu.setApplicationMenu(menu);
//   }

//   registerHotkeys() {
//     // Try to register global shortcuts
//     try {
//       // Unregister any existing shortcuts
//       globalShortcut.unregisterAll();
      
//       // Toggle overlay visibility
//       const toggleRet = globalShortcut.register(currentHotkeys.toggleVisibility, () => {
//         console.log('Toggle visibility shortcut triggered!');
//         if (overlayWindow) {
//           if (overlayWindow.isVisible()) {
//             console.log('Hiding overlay');
//             overlayWindow.hide();
//             manuallyHidden = true;
//           } else {
//             console.log('Showing overlay');
//             overlayWindow.show();
//             overlayWindow.setAlwaysOnTop(true, 'screen-saver');
//             overlayWindow.focus();
//             manuallyHidden = false;
//           }
//         }
//       });

//       // Toggle interactive mode
//       const interactiveRet = globalShortcut.register(currentHotkeys.toggleInteractive, () => {
//         console.log('Toggle interactive mode triggered!');
//         if (overlayWindow && overlayWindow.isVisible()) {
//           isInteractive = !isInteractive;
//           overlayWindow.setIgnoreMouseEvents(!isInteractive);
          
//           // Send state to renderer for visual feedback
//           overlayWindow.webContents.send('interactive-mode-changed', isInteractive);
          
//           console.log('Interactive mode:', isInteractive ? 'ON' : 'OFF');
//         }
//       });

//       if (!toggleRet) {
//         console.log('Failed to register toggle shortcut');
//       } else {
//         console.log('Toggle shortcut registered successfully:', currentHotkeys.toggleVisibility);
//       }

//       if (!interactiveRet) {
//         console.log('Failed to register interactive shortcut');
//       } else {
//         console.log('Interactive shortcut registered successfully:', currentHotkeys.toggleInteractive);
//       }

//     } catch (error) {
//       console.error('Error registering global shortcuts:', error);
//     }
//   }

//   startGameMonitoring() {
//     gameMonitorInterval = setInterval(() => {
//       this.checkForPoE2();
//     }, 3000);
//   }

//   checkForPoE2() {
//     exec('tasklist /fo csv | findstr /i "PathOfExile"', (error, stdout) => {
//       const wasRunning = isGameRunning;
//       isGameRunning = stdout.includes('PathOfExile');
      
//       if (isGameRunning && !wasRunning && !manuallyHidden) {
//         console.log('PoE2 detected - showing overlay in click-through mode');
//         isInteractive = false; // Start in click-through mode when game starts
//         overlayWindow.show();
//         overlayWindow.setAlwaysOnTop(true, 'screen-saver');
//         overlayWindow.setIgnoreMouseEvents(true); // Click-through by default
//         overlayWindow.webContents.send('interactive-mode-changed', false);
//       } else if (!isGameRunning && wasRunning) {
//         console.log('PoE2 closed - hiding overlay');
//         overlayWindow.hide();
//         manuallyHidden = false; // Reset manual state when game closes
//         isInteractive = false; // Reset interactive state
//       }
      
//       // Ensure overlay stays on top if game is running and overlay is visible
//       if (isGameRunning && overlayWindow.isVisible()) {
//         overlayWindow.setAlwaysOnTop(true, 'screen-saver');
//       }
//     });
//   }

//   setupIPC() {
//     // Load current tasks
//     ipcMain.handle('load-tasks', async () => {
//       try {
//         const data = await fs.readFile(TASKS_FILE, 'utf8');
//         return JSON.parse(data);
//       } catch (error) {
//         return { tasks: [], currentTemplate: null };
//       }
//     });

//     // Save current tasks
//     ipcMain.handle('save-tasks', async (event, tasksData) => {
//       try {
//         await fs.writeFile(TASKS_FILE, JSON.stringify(tasksData, null, 2));
//         return { success: true };
//       } catch (error) {
//         console.error('Failed to save tasks:', error);
//         return { success: false, error: error.message };
//       }
//     });

//     // Load templates
//     ipcMain.handle('load-templates', async () => {
//       try {
//         const data = await fs.readFile(TEMPLATES_FILE, 'utf8');
//         return JSON.parse(data);
//       } catch (error) {
//         return [];
//       }
//     });

//     // Save templates
//     ipcMain.handle('save-templates', async (event, templates) => {
//       try {
//         await fs.writeFile(TEMPLATES_FILE, JSON.stringify(templates, null, 2));
//         return { success: true };
//       } catch (error) {
//         console.error('Failed to save templates:', error);
//         return { success: false, error: error.message };
//       }
//     });

//     // Save settings
//     ipcMain.handle('save-settings', async (event, settings) => {
//       try {
//         console.log('Attempting to save settings:', settings);
//         console.log('Settings file path:', SETTINGS_FILE);
        
//         // Ensure directory exists
//         await fs.mkdir(DATA_DIR, { recursive: true });
        
//         const settingsJson = JSON.stringify(settings, null, 2);
//         console.log('Settings JSON:', settingsJson);
        
//         await fs.writeFile(SETTINGS_FILE, settingsJson);
//         console.log('Settings saved successfully to:', SETTINGS_FILE);
        
//         return { success: true };
//       } catch (error) {
//         console.error('Failed to save settings:', error);
//         return { success: false, error: error.message };
//       }
//     });

//     // Load settings
//     ipcMain.handle('load-settings', async () => {
//       try {
//         console.log('Attempting to load settings from:', SETTINGS_FILE);
//         const data = await fs.readFile(SETTINGS_FILE, 'utf8');
//         const settings = JSON.parse(data);
//         console.log('Settings loaded successfully:', settings);
//         return settings;
//       } catch (error) {
//         console.log('Settings file not found or invalid, using defaults:', error.message);
//         return null;
//       }
//     });

//     // Update hotkeys
//     ipcMain.on('update-hotkeys', (event, hotkeys) => {
//       console.log('Received hotkey update request:', hotkeys);
      
//       // Convert hotkeys to Electron format
//       const newToggleVisibility = this.convertHotkeyFormat(hotkeys.toggleVisibility);
//       const newToggleInteractive = this.convertHotkeyFormat(hotkeys.toggleInteractive);
      
//       console.log('Converted hotkeys:', {
//         visibility: newToggleVisibility,
//         interactive: newToggleInteractive
//       });
      
//       // Only update if they're different
//       if (newToggleVisibility !== currentHotkeys.toggleVisibility || 
//           newToggleInteractive !== currentHotkeys.toggleInteractive) {
        
//         currentHotkeys.toggleVisibility = newToggleVisibility;
//         currentHotkeys.toggleInteractive = newToggleInteractive;
        
//         // Re-register hotkeys with new bindings
//         this.registerHotkeys();
//       }
//     });

//     // Reset window position
//     ipcMain.on('reset-window-position', () => {
//       if (overlayWindow) {
//         const primaryDisplay = screen.getPrimaryDisplay();
//         const { width } = primaryDisplay.workAreaSize;
//         overlayWindow.setPosition(width - 370, 20);
//       }
//     });

//     // Handle window movement
//     ipcMain.on('move-window', (event, { x, y }) => {
//       if (overlayWindow) {
//         overlayWindow.setPosition(x, y);
//       }
//     });

//     // Manual show/hide for testing
//     ipcMain.on('toggle-overlay', () => {
//       console.log('IPC: toggle-overlay received');
//       if (overlayWindow) {
//         if (overlayWindow.isVisible()) {
//           console.log('IPC: Hiding overlay');
//           overlayWindow.hide();
//           manuallyHidden = true;
//         } else {
//           console.log('IPC: Showing overlay');
//           overlayWindow.show();
//           overlayWindow.setAlwaysOnTop(true, 'screen-saver');
//           overlayWindow.focus();
//           manuallyHidden = false;
//         }
//       }
//     });

//     // Minimize overlay
//     ipcMain.on('minimize-overlay', () => {
//       console.log('IPC: minimize-overlay received');
//       if (overlayWindow) {
//         console.log('IPC: Hiding overlay');
//         overlayWindow.hide();
//         manuallyHidden = true;
//       }
//     });

//     // Show overlay
//     ipcMain.on('show-overlay', () => {
//       console.log('IPC: show-overlay received');
//       if (overlayWindow) {
//         overlayWindow.show();
//         overlayWindow.setAlwaysOnTop(true, 'screen-saver');
//         overlayWindow.focus();
//         manuallyHidden = false;
//       }
//     });

//     // Toggle interactive mode
//     ipcMain.on('toggle-interactive-mode', () => {
//       console.log('IPC: toggle-interactive-mode received');
//       if (overlayWindow && overlayWindow.isVisible()) {
//         isInteractive = !isInteractive;
//         overlayWindow.setIgnoreMouseEvents(!isInteractive);
//         overlayWindow.webContents.send('interactive-mode-changed', isInteractive);
//         console.log('Interactive mode:', isInteractive ? 'ON' : 'OFF');
//       }
//     });

//     // Quit application
//     ipcMain.on('quit-application', () => {
//       console.log('IPC: quit-application received');
//       this.cleanup();
//       app.quit();
//     });
//   }

//   convertHotkeyFormat(userHotkey) {
//     // Convert user-friendly format to Electron format
//     // e.g., "Ctrl+Shift+T" -> "CommandOrControl+Shift+T"
//     if (!userHotkey) return 'CommandOrControl+Shift+T';
    
//     console.log('Converting hotkey:', userHotkey);
    
//     // Only replace if it doesn't already contain CommandOrControl
//     if (userHotkey.includes('CommandOrControl')) {
//       console.log('Already in Electron format:', userHotkey);
//       return userHotkey;
//     }
    
//     let converted = userHotkey
//       .replace(/\bCtrl\b/gi, 'CommandOrControl')
//       .replace(/\bCmd\b/gi, 'CommandOrControl')
//       .replace(/\bCommand\b/gi, 'CommandOrControl');
    
//     console.log('Converted to:', converted);
//     return converted;
//   }

//   cleanup() {
//     if (gameMonitorInterval) {
//       clearInterval(gameMonitorInterval);
//     }
//     // Unregister global shortcuts
//     globalShortcut.unregisterAll();
//   }
// }

// // Initialize overlay
// const overlay = new PoE2TaskOverlay();

// app.whenReady().then(() => {
//   overlay.initialize();
// });

// app.on('window-all-closed', () => {
//   overlay.cleanup();
//   if (process.platform !== 'darwin') {
//     app.quit();
//   }
// });

// app.on('activate', () => {
//   if (BrowserWindow.getAllWindows().length === 0) {
//     overlay.createOverlayWindow();
//   }
// });