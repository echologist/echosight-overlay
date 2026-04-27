# Echosight - PoE Task Overlay

A lightweight, customizable task management overlay designed specifically for Path of Exile players. Keep track of your league start goals, character progression, and endgame objectives without ever leaving the game!


## ✨ Features

### 🎯 **Smart Overlay System**
- **Click-Through Mode**: Transparent overlay that doesn't interfere with gameplay
- **Interactive Mode**: Full-featured task management when you need it
- **Auto-Detection**: Automatically appears when PoE is running
- **Always On Top**: Stays visible during intense gameplay moments

### 📋 **Task Management**
- ✅ Add custom tasks for any situation
- ✅ Visual progress tracking with completion percentage
- ✅ Check off completed objectives with satisfying feedback
- ✅ Delete unwanted or outdated tasks
- ✅ Undo recent task actions with `Ctrl+Shift+Z` and forward them again with `Ctrl+Shift+Y`
- ✅ Clean, minimal interface that doesn't clutter your screen
- ✅ Hierarchical sub-tasks with parent/child relationships
- ✅ Drag-and-drop reordering

### ⚡ **Background Tasks (New in v1.1.0)**
- 🔄 Create **background tasks** that activate when a main task is completed
- 🔗 Configure **triggers** on any task via right-click > "Configure Triggers"
- 📌 Background tasks appear in a separate section below the main task list
- 🎯 Main chain progression (including the Ctrl+Shift+N hotkey) is unaffected
- 🔴 High-priority background tasks get a visual pulsing indicator
- ⏱️ Optional expiration timers for time-limited objectives
- 🔔 Notification toast when background tasks activate

### 📁 **Template System**
- 💾 Save task lists as reusable templates
- 🔄 Load pre-made templates for different scenarios
- 📤 Export templates to share with friends
- 📥 Import templates from the community
- 🌟 Built-in community templates for common goals

### ⚙️ **Fully Customizable**
- 🎨 Adjustable transparency (10% to 70% visibility)
- 🎨 Multiple background styles (Dark, Light, Transparent)
- ⌨️ Custom hotkeys with intuitive recording system
- 📍 Draggable window positioning
- 💾 All settings persist between sessions

### 🌟 **Built-in Community Templates**
- **League Start Essentials**: Core early-game objectives
- **Endgame Progression**: Pinnacle bosses and atlas goals  
- **New Character Setup**: Build planning checklist
- **Currency Goals**: Economic milestones
- **HC/SSF Priorities**: Hardcore-focused objectives
- **Crafting Checklist**: Gear progression steps

## 🚀 Quick Start

### Build from Source
```bash
# Clone the repository
git clone https://github.com/echologist/echosight-overlay.git
cd echosight-overlay

# Install dependencies
npm install

# Start development with hot reload
npm run dev

# Build for production
npm run build

# Test production build
npm run preview

# Create distributable package
npm run pack

# Type-check the TypeScript code
npm run typecheck
```

## 🎮 Usage Guide

### Basic Controls
- **Show/Hide Overlay**: `Ctrl+Shift+T` (customizable)
- **Toggle Interactive Mode**: `Ctrl+Shift+I` (customizable)
- **Add Task**: Type in input field and press Enter
- **Complete Task**: Click the checkbox `Ctrl+Shift+N` (customizable)
- **Undo Last Task Action**: `Ctrl+Shift+Z` (customizable)
- **Forward Last Task Action**: `Ctrl+Shift+Y` (customizable)
- **Delete Task**: Click the × button next to any task and confirm

### Two Modes Explained

#### 🌫️ Click-Through Mode (Default)
Perfect for gaming - you see your tasks but can't accidentally click on them:
- Semi-transparent overlay
- Mouse clicks pass through to the game
- Shows only essential information (tasks + progress)
- No distracting buttons or controls

#### ⚡ Interactive Mode  
Full functionality when you need to manage tasks:
- Interactive overlay
- Add, edit, and organize tasks
- Access settings and templates
- Import/export functionality

### Template System

#### Creating Templates
1. Add your desired tasks
2. Click "Save" in template section
3. Enter a descriptive name
4. Template is saved for future use

#### Using Community Templates
1. Click "Community Templates"
2. Browse available templates
3. Click "Add to My Templates"
4. Load whenever starting a new character/league

#### Sharing Templates
1. Select your template
2. Click "Export" to download JSON file
3. Share the file with friends
4. Others can import using "Import" button

## ⚙️ Customization

### Appearance Settings
- **Transparency**: Adjust from 10% (barely visible) to 70% (more solid)
- **Background Style**: 
  - Dark (default black background)
  - Light (white background with dark text)
  - Transparent (text only with shadows) (fake news)

### Hotkey Customization
- Click "Record" next to any hotkey
- Press your desired key combination
- Avoid single letters (like Shift+I) to prevent typing conflicts
- Good examples: `Ctrl+F1`, `Alt+Q`, `Ctrl+Shift+T`

### Background Tasks
Background tasks are objectives that become relevant after completing a specific task but shouldn't interrupt your main progression. For example, after completing Act 2, you might want to "Watch for 4-linked items" — this stays visible as a background task while you continue through the main quest line.

#### Setting Up Background Tasks
1. Right-click any task and select "Configure Triggers"
2. Create new background tasks using the input field in the modal
3. Check the background tasks you want to link as triggers
4. Click "Save" — a lightning bolt icon appears on the trigger task
5. When you complete the trigger task, the linked background tasks activate and appear in the "Background Tasks" section

#### How They Work
- Background tasks start **dormant** (invisible) until triggered
- Once activated, they appear in a separate section below your main tasks
- They can be completed independently at any time
- The `Ctrl+Shift+N` hotkey only advances main chain tasks
- The progress bar counts both main and active background tasks
- Templates preserve background task configurations

### Best Practices
- Use `Ctrl+Key` or `Alt+Key` combinations
- Avoid conflicts with PoE's built-in shortcuts
- Function keys (F1-F12) work well with modifiers

## 🔧 Technical Details

### Built With
- **Electron**: Cross-platform desktop app framework
- **Node.js**: JavaScript runtime
- **Native Windows APIs**: For overlay functionality and hotkey registration

### System Requirements
- Windows 10/11, macOS, or Linux (64-bit)
- ~150MB disk space
- No additional dependencies required

### Performance
- Minimal CPU usage (~1%)
- Low memory footprint (~30MB RAM)
- No impact on game performance
- Efficient click-through detection

## 🙏 Credits

### Inspiration
- Luca lol

### Safe for Path of Exile
- ✅ **No game memory reading** - Only displays user-entered information
- ✅ **No automation** - Requires manual user input for all actions
- ✅ **No game file modification** - Pure overlay application
- ✅ **Similar to external tools** like Discord overlay, hardware monitors, etc.

### What This App Does NOT Do
- ❌ Read game data automatically
- ❌ Send keystrokes or clicks to the game
- ❌ Modify game files or memory
- ❌ Provide unfair competitive advantages
- ❌ Violate any game terms of service

*This overlay is essentially a smart notepad that floats over your game*
