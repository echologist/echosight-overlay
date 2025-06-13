# Echoesight - PoE 2 Task Overlay

A lightweight, customizable task management overlay designed specifically for Path of Exile 2 players. Keep track of your league start goals, character progression, and endgame objectives without ever leaving the game!


## ✨ Features

### 🎯 **Smart Overlay System**
- **Click-Through Mode**: Transparent overlay that doesn't interfere with gameplay
- **Interactive Mode**: Full-featured task management when you need it
- **Auto-Detection**: Automatically appears when PoE2 is running
- **Always On Top**: Stays visible during intense gameplay moments

### 📋 **Task Management**
- ✅ Add custom tasks for any situation
- ✅ Visual progress tracking with completion percentage
- ✅ Check off completed objectives with satisfying feedback
- ✅ Delete unwanted or outdated tasks
- ✅ Clean, minimal interface that doesn't clutter your screen

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
git clone https://github.com/echologist/echoesight-overlay.git
cd echoesight-overlay

# Install dependencies
yarn install

# Run in development mode
yarn start

# Build executable
yarn build-win
```

## 🎮 Usage Guide

### Basic Controls
- **Show/Hide Overlay**: `Ctrl+Shift+T` (customizable)
- **Toggle Interactive Mode**: `Ctrl+Shift+I` (customizable)
- **Add Task**: Type in input field and press Enter
- **Complete Task**: Click the checkbox
- **Delete Task**: Click the × button next to any task

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

### Best Practices
- Use `Ctrl+Key` or `Alt+Key` combinations
- Avoid conflicts with PoE2's built-in shortcuts
- Function keys (F1-F12) work well with modifiers

## 🔧 Technical Details

### Built With
- **Electron**: Cross-platform desktop app framework
- **Node.js**: JavaScript runtime
- **Native Windows APIs**: For overlay functionality and hotkey registration

### System Requirements
- Windows 10/11 (64-bit)
- ~150MB disk space
- No additional dependencies required

### Performance
- Minimal CPU usage (~1%)
- Low memory footprint (~30MB RAM)
- No impact on game performance
- Efficient click-through detection

## 🙏 Credits

### Inspiration
Luca lol

### Safe for Path of Exile 2
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
