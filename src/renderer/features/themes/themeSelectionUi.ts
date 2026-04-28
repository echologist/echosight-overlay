import type {
  EchosightApi,
  Theme
} from '../../../shared/types';
import { hideModal, showModal } from '../../ui/modalUi';
import { generateThemeCSS } from './themeCss';
import {
  loadThemeAssets,
  loadThemeCss
} from './themeRuntime';

const THEME_SELECTION_MODAL_ID = 'themeSelectionModal';
const PREVIEW_ROOT_SELECTOR = '#themeSelectionPreviewRoot';
const PREVIEW_STYLE_ID = 'theme-selection-preview-style';
const COLOR_SWATCHES = [
  ['Primary', ['colors', 'primary']],
  ['Secondary', ['colors', 'secondary']],
  ['Danger', ['colors', 'danger']],
  ['Text', ['colors', 'text', 'primary']],
  ['Surface', ['colors', 'background', 'primary']],
  ['Border', ['colors', 'border', 'primary']]
] as const;

type ThemeSelectionPreviewApi = Pick<EchosightApi, 'getThemeAsset' | 'loadThemeCss'>;
type ThemeApplyHandler = (themeId: string) => Promise<void> | void;

export interface ThemeSelectionOptions {
  api: ThemeSelectionPreviewApi;
  currentThemeId: string;
  onApply: ThemeApplyHandler;
  themes: Theme[];
  transparency: number;
}

interface ThemeSelectionState extends ThemeSelectionOptions {
  previewThemeId: string;
  previewRequestId: number;
}

let selectionState: ThemeSelectionState | null = null;

export function showThemeSelectionModal(options: ThemeSelectionOptions): void {
  const previewThemeId = getInitialThemeId(options.themes, options.currentThemeId);
  selectionState = {
    ...options,
    previewThemeId,
    previewRequestId: 0
  };

  bindThemeSelectionActions();
  renderThemeSelectionList();
  void previewTheme(previewThemeId);

  showModal(THEME_SELECTION_MODAL_ID, {
    focusSelector: `[data-theme-selection-id="${cssEscape(previewThemeId)}"]`,
    restoreFocus: true
  });
}

function bindThemeSelectionActions(): void {
  const applyButton = document.getElementById('applyThemeSelectionButton');
  if (applyButton instanceof HTMLButtonElement) {
    applyButton.onclick = () => {
      void applyPreviewedTheme();
    };
  }
}

export function hideThemeSelectionModal(): void {
  hideModal(THEME_SELECTION_MODAL_ID);
  document.getElementById(PREVIEW_STYLE_ID)?.remove();
  selectionState = null;
}

export function renderThemeSelectionList(): void {
  const state = selectionState;
  const container = document.getElementById('themeSelectionList');
  if (!state || !container) {
    return;
  }

  container.replaceChildren(...state.themes.map(theme => createThemeSelectionButton(theme, state)));
  updateApplyButtonState();
}

export async function previewTheme(themeId: string): Promise<void> {
  const state = selectionState;
  if (!state) {
    return;
  }

  const theme = state.themes.find(candidate => candidate.id === themeId);
  if (!theme) {
    return;
  }

  state.previewThemeId = theme.id;
  const requestId = state.previewRequestId + 1;
  state.previewRequestId = requestId;

  renderThemeSelectionList();
  renderThemeDetails(theme, state.transparency);
  renderLoadingPreviewState(true);

  try {
    const css = await buildScopedPreviewCss(theme, state);
    if (selectionState?.previewRequestId !== requestId) {
      return;
    }

    replacePreviewStyle(css);
  } finally {
    if (selectionState?.previewRequestId === requestId) {
      renderLoadingPreviewState(false);
    }
  }
}

export async function applyPreviewedTheme(): Promise<void> {
  const state = selectionState;
  if (!state) {
    return;
  }

  const themeId = state.previewThemeId;
  if (!themeId) {
    return;
  }

  const applyButton = document.getElementById('applyThemeSelectionButton');
  if (applyButton instanceof HTMLButtonElement) {
    applyButton.disabled = true;
  }

  await state.onApply(themeId);
  hideThemeSelectionModal();
}

export function scopeThemeCss(css: string, scopeSelector = PREVIEW_ROOT_SELECTOR): string {
  const blocks: string[] = [];
  let index = 0;

  while (index < css.length) {
    const openingBraceIndex = css.indexOf('{', index);
    if (openingBraceIndex < 0) {
      const statement = css.slice(index).trim();
      if (statement) {
        blocks.push(statement);
      }
      break;
    }

    const preludeChunk = css.slice(index, openingBraceIndex);
    const closingBraceIndex = findMatchingBrace(css, openingBraceIndex);
    if (closingBraceIndex < 0) {
      break;
    }

    const { leadingStatements, selectorPrelude } = splitLeadingCssStatements(preludeChunk);
    if (leadingStatements) {
      blocks.push(leadingStatements);
    }

    const body = css.slice(openingBraceIndex + 1, closingBraceIndex);
    const scopedBlock = scopeCssBlock(selectorPrelude, body, scopeSelector);
    if (scopedBlock) {
      blocks.push(scopedBlock);
    }

    index = closingBraceIndex + 1;
  }

  return blocks.join('\n');
}

function createThemeSelectionButton(theme: Theme, state: ThemeSelectionState): HTMLButtonElement {
  const button = document.createElement('button');
  button.type = 'button';
  button.className = 'theme-selection-item';
  button.dataset.themeSelectionId = theme.id;
  button.classList.toggle('is-previewed', theme.id === state.previewThemeId);
  button.classList.toggle('is-current', theme.id === state.currentThemeId);

  const name = document.createElement('span');
  name.className = 'theme-selection-item-name';
  name.textContent = theme.name;

  const meta = document.createElement('span');
  meta.className = 'theme-selection-item-meta';
  meta.textContent = getThemeListMeta(theme, state.currentThemeId);

  button.append(name, meta);
  button.addEventListener('click', () => {
    void previewTheme(theme.id);
  });

  return button;
}

function renderThemeDetails(theme: Theme, transparency: number): void {
  setText('themeSelectionName', theme.name);
  setText('themeSelectionDescription', readString(theme.description) || 'No description provided.');
  setText('themeSelectionSampleThemeName', theme.name);
  renderThemeMetadata(theme, transparency);
  renderThemeSwatches(theme);
}

function renderThemeMetadata(theme: Theme, transparency: number): void {
  const container = document.getElementById('themeSelectionMetadata');
  if (!container) {
    return;
  }

  container.replaceChildren(
    createMetadataRow('ID', theme.id),
    createMetadataRow('Author', readString(theme.author) || 'Unknown'),
    createMetadataRow('Version', readString(theme.version) || 'Unversioned'),
    createMetadataRow(
      'Transparency',
      theme.supportsTransparency === false ? 'Not supported' : `${transparency}% visible`
    ),
    createMetadataRow('Assets', String(Object.keys(theme.assets || {}).length)),
    createMetadataRow('CSS', readString(theme.cssFile) || 'Generated')
  );
}

function renderThemeSwatches(theme: Theme): void {
  const container = document.getElementById('themeSelectionSwatches');
  if (!container) {
    return;
  }

  container.replaceChildren(...COLOR_SWATCHES.map(([label, path]) => {
    const value = readThemePath(theme, path);
    return createSwatch(label, value);
  }));
}

async function buildScopedPreviewCss(theme: Theme, state: ThemeSelectionState): Promise<string> {
  const previewTheme = structuredClone(theme) as Theme;
  await loadThemeAssets(previewTheme, state.api.getThemeAsset);
  const cssFileStyles = await loadThemeCss(previewTheme, state.api.loadThemeCss);
  return scopeThemeCss(generateThemeCSS(previewTheme, state.transparency / 100, cssFileStyles));
}

function replacePreviewStyle(css: string): void {
  let style = document.getElementById(PREVIEW_STYLE_ID);
  if (!(style instanceof HTMLStyleElement)) {
    style = document.createElement('style');
    style.id = PREVIEW_STYLE_ID;
    document.head.appendChild(style);
  }

  style.textContent = css;
}

function renderLoadingPreviewState(loading: boolean): void {
  const root = document.getElementById('themeSelectionPreviewRoot');
  root?.classList.toggle('is-loading-theme-preview', loading);
}

function updateApplyButtonState(): void {
  const state = selectionState;
  const applyButton = document.getElementById('applyThemeSelectionButton');
  if (!(applyButton instanceof HTMLButtonElement) || !state) {
    return;
  }

  applyButton.disabled = !state.previewThemeId || state.previewThemeId === state.currentThemeId;
}

function createMetadataRow(label: string, value: string): HTMLElement {
  const row = document.createElement('div');
  row.className = 'theme-selection-meta-row';

  const labelElement = document.createElement('span');
  labelElement.className = 'theme-selection-meta-label';
  labelElement.textContent = label;

  const valueElement = document.createElement('span');
  valueElement.className = 'theme-selection-meta-value';
  valueElement.textContent = value;

  row.append(labelElement, valueElement);
  return row;
}

function createSwatch(label: string, value: string): HTMLElement {
  const swatch = document.createElement('div');
  swatch.className = 'theme-selection-swatch';

  const color = document.createElement('span');
  color.className = 'theme-selection-swatch-color';
  if (value) {
    color.style.background = value;
  }

  const text = document.createElement('span');
  text.className = 'theme-selection-swatch-text';
  text.textContent = value ? `${label}: ${value}` : `${label}: unset`;

  swatch.append(color, text);
  return swatch;
}

function scopeCssBlock(selectorPrelude: string, body: string, scopeSelector: string): string {
  const selectors = stripCssComments(selectorPrelude).trim();
  if (!selectors || !body.trim()) {
    return '';
  }

  if (selectors.startsWith('@keyframes') || selectors.startsWith('@font-face')) {
    return `${selectors} {${body}}`;
  }

  if (selectors.startsWith('@')) {
    return `${selectors} {\n${scopeThemeCss(body, scopeSelector)}\n}`;
  }

  const scopedSelectors = selectors
    .split(',')
    .map(selector => scopeSelectorText(selector.trim(), scopeSelector))
    .filter(Boolean)
    .join(', ');

  return scopedSelectors
    ? `${scopedSelectors} {${body}}`
    : '';
}

function findMatchingBrace(css: string, openingBraceIndex: number): number {
  let depth = 0;
  for (let index = openingBraceIndex; index < css.length; index += 1) {
    const char = css[index];
    if (char === '{') {
      depth += 1;
    } else if (char === '}') {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }

  return -1;
}

function splitLeadingCssStatements(preludeChunk: string): {
  leadingStatements: string;
  selectorPrelude: string;
} {
  const lastSemicolonIndex = preludeChunk.lastIndexOf(';');
  if (lastSemicolonIndex < 0) {
    return {
      leadingStatements: '',
      selectorPrelude: preludeChunk
    };
  }

  return {
    leadingStatements: preludeChunk.slice(0, lastSemicolonIndex + 1).trim(),
    selectorPrelude: preludeChunk.slice(lastSemicolonIndex + 1)
  };
}

function stripCssComments(value: string): string {
  return value.replace(/\/\*[\s\S]*?\*\//g, '');
}

function scopeSelectorText(selector: string, scopeSelector: string): string {
  if (!selector || selector.startsWith(scopeSelector)) {
    return selector;
  }

  if (selector === ':root' || selector === 'html' || selector === 'body') {
    return scopeSelector;
  }

  if (selector.startsWith('body ')) {
    return `${scopeSelector} ${selector.slice(5)}`;
  }

  return `${scopeSelector} ${selector}`;
}

function getInitialThemeId(themes: Theme[], currentThemeId: string): string {
  return themes.some(theme => theme.id === currentThemeId)
    ? currentThemeId
    : themes[0]?.id ?? '';
}

function getThemeListMeta(theme: Theme, currentThemeId: string): string {
  const parts = [
    theme.id === currentThemeId ? 'Current' : '',
    readString(theme.author),
    readString(theme.version)
  ].filter(Boolean);

  return parts.join(' - ') || theme.id;
}

function setText(id: string, value: string): void {
  const element = document.getElementById(id);
  if (element) {
    element.textContent = value;
  }
}

function readThemePath(theme: Theme, path: readonly string[]): string {
  let current: unknown = theme;
  for (const segment of path) {
    if (!isRecord(current)) {
      return '';
    }
    current = current[segment];
  }

  return typeof current === 'string' || typeof current === 'number'
    ? String(current)
    : '';
}

function readString(value: unknown): string {
  return typeof value === 'string' ? value.trim() : '';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return !!value && typeof value === 'object' && !Array.isArray(value);
}

function cssEscape(value: string): string {
  const cssApi = window.CSS;
  if (cssApi && typeof cssApi.escape === 'function') {
    return cssApi.escape(value);
  }

  return value.replace(/["\\]/g, '\\$&');
}
