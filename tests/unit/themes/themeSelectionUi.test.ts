import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import {
  hideThemeSelectionModal,
  scopeThemeCss,
  showThemeSelectionModal
} from '../../../src/renderer/features/themes/themeSelectionUi';
import type {
  EchosightApi,
  Theme
} from '../../../src/shared/types';

describe('theme selection UI', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    renderSelectionFixture();
  });

  afterEach(() => {
    hideThemeSelectionModal();
    document.getElementById('theme-selection-preview-style')?.remove();
    vi.useRealTimers();
    vi.restoreAllMocks();
    document.body.replaceChildren();
  });

  test('browses themes without applying them', async () => {
    const onApply = vi.fn();

    showThemeSelectionModal({
      api: previewApi,
      currentThemeId: 'echosight',
      onApply,
      themes: themeFixtures,
      transparency: 70
    });
    await allowPreviewToRender();

    expect(getModal().classList.contains('is-visible')).toBe(true);
    expect(readThemeButtons()).toEqual([
      ['Echosight', 'Current - Echologist - 2.0.0'],
      ['Dark', 'Dark Theme - 1.5.0']
    ]);
    expect(getElement('themeSelectionName').textContent).toBe('Echosight');
    expect(getButton('applyThemeSelectionButton').disabled).toBe(true);

    getThemeButton('dark').click();
    await allowPreviewToRender();

    expect(onApply).not.toHaveBeenCalled();
    expect(getElement('themeSelectionName').textContent).toBe('Dark');
    expect(getElement('themeSelectionDescription').textContent).toBe('Classic dark theme');
    expect(getElement('themeSelectionMetadata').textContent).toContain('Dark Theme');
    expect(getElement('themeSelectionSwatches').textContent).toContain('Primary: #d4af37');
    expect(getButton('applyThemeSelectionButton').disabled).toBe(false);
  });

  test('applies the previewed theme only when Apply is clicked', async () => {
    const onApply = vi.fn();

    showThemeSelectionModal({
      api: previewApi,
      currentThemeId: 'echosight',
      onApply,
      themes: themeFixtures,
      transparency: 70
    });
    await allowPreviewToRender();

    getThemeButton('dark').click();
    await allowPreviewToRender();
    getButton('applyThemeSelectionButton').click();
    await allowPreviewToRender();

    expect(onApply).toHaveBeenCalledWith('dark');
    expect(getModal().classList.contains('is-visible')).toBe(false);
  });

  test('scopes generated theme css to the preview root', () => {
    expect(scopeThemeCss(`
      :root { --primary: red; }
      body { background: black; }
      .overlay-container.interactive { border-color: red; }
      button, .template-btn { color: red; }
    `)).toContain('#themeSelectionPreviewRoot .overlay-container.interactive');

    expect(scopeThemeCss(':root { --primary: red; }')).toContain('#themeSelectionPreviewRoot { --primary: red; }');
    expect(scopeThemeCss('body { background: black; }')).toContain('#themeSelectionPreviewRoot { background: black; }');
    expect(scopeThemeCss('button, .template-btn { color: red; }'))
      .toContain('#themeSelectionPreviewRoot button, #themeSelectionPreviewRoot .template-btn');
    expect(scopeThemeCss('@media (min-width: 400px) { body { color: red; } }'))
      .toContain('@media (min-width: 400px) {\n#themeSelectionPreviewRoot { color: red; }');
    expect(scopeThemeCss('@keyframes pulse { 0% { opacity: 0; } 100% { opacity: 1; } }'))
      .toContain('@keyframes pulse');
  });
});

const themeFixtures: Theme[] = [
  {
    id: 'echosight',
    name: 'Echosight',
    description: 'Default theme',
    author: 'Echologist',
    version: '2.0.0',
    supportsTransparency: true,
    colors: {
      primary: '#d4a857',
      secondary: '#5fd8c8',
      danger: '#c54a3a',
      text: {
        primary: '#ffffff'
      },
      background: {
        primary: 'rgba(0, 0, 0, 0.7)'
      },
      border: {
        primary: '#d4a857'
      }
    }
  },
  {
    id: 'dark',
    name: 'Dark',
    description: 'Classic dark theme',
    author: 'Dark Theme',
    version: '1.5.0',
    cssFile: 'styles.css',
    supportsTransparency: true,
    colors: {
      primary: '#d4af37',
      secondary: '#32cd32',
      danger: '#dc143c',
      text: {
        primary: '#ffffff'
      },
      background: {
        primary: 'rgba(0, 0, 0, 0.8)'
      },
      border: {
        primary: '#d4af37'
      }
    }
  }
];

const previewApi: Pick<EchosightApi, 'getThemeAsset' | 'loadThemeCss'> = {
  getThemeAsset: async () => null,
  loadThemeCss: async () => '.task-text { font-weight: 700; }'
};

function renderSelectionFixture(): void {
  document.body.innerHTML = `
    <button id="openThemeSelectionButton">Open</button>
    <div id="themeSelectionModal" class="modal">
      <div class="modal-content">
        <div id="themeSelectionList"></div>
        <h3 id="themeSelectionName"></h3>
        <p id="themeSelectionDescription"></p>
        <span id="themeSelectionSampleThemeName"></span>
        <div id="themeSelectionPreviewRoot"></div>
        <div id="themeSelectionMetadata"></div>
        <div id="themeSelectionSwatches"></div>
        <button id="applyThemeSelectionButton">Apply</button>
        <button id="closeThemeSelectionButton">Close</button>
      </div>
    </div>
  `;
}

async function allowPreviewToRender(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  vi.runOnlyPendingTimers();
}

function readThemeButtons(): string[][] {
  return Array.from(document.querySelectorAll('.theme-selection-item'))
    .map(button => [
      button.querySelector('.theme-selection-item-name')?.textContent || '',
      button.querySelector('.theme-selection-item-meta')?.textContent || ''
    ]);
}

function getThemeButton(themeId: string): HTMLButtonElement {
  const button = document.querySelector(`[data-theme-selection-id="${themeId}"]`);
  if (!(button instanceof HTMLButtonElement)) {
    throw new Error(`Missing theme button ${themeId}`);
  }
  return button;
}

function getModal(): HTMLElement {
  return getElement('themeSelectionModal');
}

function getButton(id: string): HTMLButtonElement {
  const element = getElement(id);
  if (!(element instanceof HTMLButtonElement)) {
    throw new Error(`Expected button ${id}`);
  }
  return element;
}

function getElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (!element) {
    throw new Error(`Missing element ${id}`);
  }
  return element;
}
