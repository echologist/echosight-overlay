import { beforeEach, describe, expect, test, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import path from 'node:path';

const electronMocks = vi.hoisted(() => {
  const tray = {
    destroy: vi.fn(),
    on: vi.fn(),
    setContextMenu: vi.fn(),
    setToolTip: vi.fn()
  };

  return {
    buildFromTemplate: vi.fn((template) => template),
    tray,
    Tray: vi.fn(function TrayMock() {
      return tray;
    })
  };
});

vi.mock('electron', () => ({
  Menu: {
    buildFromTemplate: electronMocks.buildFromTemplate
  },
  Tray: electronMocks.Tray
}));

import {
  createAppTray,
  getTrayIconPath,
  shouldCreateAppTray
} from '../../../src/main/tray/appTray';

describe('Echosight app tray', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  test('provides show, hide, and exit actions', () => {
    const onShow = vi.fn();
    const onHide = vi.fn();
    const onExit = vi.fn();

    const tray = createAppTray({
      iconPath: '/app/icon.png',
      onShow,
      onHide,
      onExit
    });

    expect(electronMocks.Tray).toHaveBeenCalledWith('/app/icon.png');
    expect(tray).toBe(electronMocks.tray);
    expect(electronMocks.tray.setToolTip).toHaveBeenCalledWith('Echosight');

    const template = electronMocks.buildFromTemplate.mock.calls[0]?.[0];
    expect(template.map((item: { label?: string; type?: string }) => item.label || item.type)).toEqual([
      'Show Echosight',
      'Hide Echosight',
      'separator',
      'Exit Echosight'
    ]);

    template[0].click();
    template[1].click();
    template[3].click();

    expect(onShow).toHaveBeenCalledOnce();
    expect(onHide).toHaveBeenCalledOnce();
    expect(onExit).toHaveBeenCalledOnce();
  });

  test('shows Echosight when the tray icon is clicked', () => {
    const onShow = vi.fn();

    createAppTray({
      iconPath: '/app/icon.png',
      onShow,
      onHide: vi.fn(),
      onExit: vi.fn()
    });

    const clickListener = electronMocks.tray.on.mock.calls.find(([event]) => event === 'click')?.[1];
    expect(clickListener).toBeDefined();

    clickListener();

    expect(onShow).toHaveBeenCalledOnce();
  });

  test('uses the packaged resource path for Windows builds', () => {
    expect(getTrayIconPath({
      isPackaged: true,
      resourcesPath: 'C:\\Program Files\\Echosight\\resources',
      mainDirectory: 'unused'
    })).toBe(path.join('C:\\Program Files\\Echosight\\resources', 'tray-icon.ico'));
  });

  test('uses the project asset while developing', () => {
    expect(getTrayIconPath({
      isPackaged: false,
      resourcesPath: 'unused',
      mainDirectory: '/project/dist-electron/main'
    })).toBe(path.join('/project/dist-electron/main', '../../assets/tray-icon.ico'));
  });

  test('packages the tray icon as a Windows runtime resource', () => {
    const packageJson = JSON.parse(
      readFileSync('package.json', 'utf8')
    );

    expect(packageJson.build.extraResources).toContainEqual({
      from: 'assets/tray-icon.ico',
      to: 'tray-icon.ico'
    });
  });

  test('provides a multi-resolution Windows icon', () => {
    const trayIcon = readFileSync('assets/tray-icon.ico');

    expect([...trayIcon.subarray(0, 4)]).toEqual([0, 0, 1, 0]);
    expect(trayIcon.readUInt16LE(4)).toBeGreaterThanOrEqual(4);
  });

  test('enables the app tray only on Windows', () => {
    expect(shouldCreateAppTray('win32')).toBe(true);
    expect(shouldCreateAppTray('darwin')).toBe(false);
    expect(shouldCreateAppTray('linux')).toBe(false);
  });
});
