import {
  isRecord,
  toKebabCase,
  type ThemeForCss
} from './themeCssHelpers';
import { layoutSelectorMap } from './themeCssSelectors';

export function generateLayoutStyles(theme: ThemeForCss): string {
  if (!isRecord(theme.layout)) return '';

  let layoutCSS = '';
  const layout = theme.layout;

  if (isRecord(layout.window)) {
    const windowLayout = layout.window;
    layoutCSS += `
      .overlay-container {
        ${windowLayout.width ? `width: ${windowLayout.width} !important;` : ''}
        ${windowLayout.height ? `height: ${windowLayout.height} !important;` : ''}
        ${windowLayout.minWidth ? `min-width: ${windowLayout.minWidth} !important;` : ''}
        ${windowLayout.minHeight ? `min-height: ${windowLayout.minHeight} !important;` : ''}
        ${windowLayout.maxWidth ? `max-width: ${windowLayout.maxWidth} !important;` : ''}
        ${windowLayout.maxHeight ? `max-height: ${windowLayout.maxHeight} !important;` : ''}
      }
    `;
  }

  if (isRecord(layout.components)) {
    Object.entries(layout.components).forEach(([component, layoutConfig]) => {
      const selectors = layoutSelectorMap[component] || [];
      if (selectors.length > 0) {
        layoutCSS += `
          ${selectors.join(', ')} {
            ${generateLayoutProperties(layoutConfig)}
          }
        `;
      }
    });
  }

  if (isRecord(layout.positions)) {
    Object.entries(layout.positions).forEach(([selector, posConfig]) => {
      layoutCSS += `
        ${selector} {
          ${generateLayoutProperties(posConfig)}
        }
      `;
    });
  }

  if (layout.grid) {
    layoutCSS += generateGridStyles(layout.grid);
  }

  if (layout.flex) {
    layoutCSS += generateFlexStyles(layout.flex);
  }

  return layoutCSS;
}

function generateLayoutProperties(layoutConfig: unknown): string {
  if (!isRecord(layoutConfig)) return '';

  const properties: string[] = [];
  const cssKeys = [
    'position', 'top', 'right', 'bottom', 'left', 'zIndex',
    'width', 'height', 'minWidth', 'minHeight', 'maxWidth', 'maxHeight',
    'margin', 'padding',
    'display', 'overflow', 'visibility',
    'flex', 'flexDirection', 'justifyContent', 'alignItems', 'flexWrap',
    'gridColumn', 'gridRow', 'gridArea'
  ];

  cssKeys.forEach(key => {
    if (layoutConfig[key]) {
      properties.push(`${toKebabCase(key)}: ${layoutConfig[key]} !important`);
    }
  });

  return properties.join(';\n      ');
}

function generateGridStyles(gridConfig: unknown): string {
  if (!isRecord(gridConfig)) return '';

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

function generateFlexStyles(flexConfig: unknown): string {
  if (!isRecord(flexConfig)) return '';

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
