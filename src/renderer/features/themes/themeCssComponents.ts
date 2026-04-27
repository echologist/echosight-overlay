import {
  isRecord,
  toKebabCase,
  type ThemeForCss
} from './themeCssHelpers';
import { componentSelectorMap } from './themeCssSelectors';

export function generateComponentStyles(theme: ThemeForCss): string {
  if (!isRecord(theme.components)) return '';

  let componentCSS = '';

  Object.entries(theme.components).forEach(([component, styles]) => {
    const selectors = componentSelectorMap[component] || [];
    if (selectors.length > 0) {
      componentCSS += generateComponentCSS(selectors, styles, theme);
    }
  });

  return componentCSS;
}

function generateComponentCSS(selectors: string[], styles: unknown, theme: ThemeForCss): string {
  if (!isRecord(styles)) return '';

  let css = '';

  if (isRecord(styles.states)) {
    Object.entries(styles.states).forEach(([state, stateStyles]) => {
      const stateSelectors = selectors.map(selector => {
        switch (state) {
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
    css += `
      ${selectors.join(', ')} {
        ${generateComponentProperties(styles, theme)}
      }
    `;
  }

  return css;
}

function generateComponentProperties(styles: unknown, theme: ThemeForCss): string {
  if (!isRecord(styles)) return '';

  const properties: string[] = [];

  Object.entries(styles).forEach(([key, value]) => {
    const cssKey = toKebabCase(key);

    if (typeof value === 'string' && value.startsWith('asset:')) {
      const assetName = value.substring(6);
      const assetUrl = theme.loadedAssets?.[assetName];
      if (!assetUrl) return;

      if (key === 'backgroundImage' || key === 'background') {
        properties.push(`background-image: url('${assetUrl}') !important`);
      } else if (key === 'borderImage') {
        properties.push(`border-image: url('${assetUrl}') !important`);
      } else {
        properties.push(`${cssKey}: url('${assetUrl}') !important`);
      }
      return;
    }

    properties.push(`${cssKey}: ${String(value)} !important`);
  });

  return properties.join(';\n      ');
}
