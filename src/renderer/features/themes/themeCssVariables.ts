import {
  isRecord,
  toKebabCase,
  type ThemeForCss
} from './themeCssHelpers';

export function generateCSSVariables(theme: ThemeForCss, transparency: number): string {
  const vars: string[] = [];

  vars.push(`--user-transparency: ${transparency}`);

  Object.entries(theme.colors || {}).forEach(([category, values]) => {
    if (isRecord(values)) {
      Object.entries(values).forEach(([key, value]) => {
        const shortCategory = category === 'background'
          ? 'bg'
          : category === 'border'
            ? 'border'
            : category === 'text'
              ? 'text'
              : category;
        const cssValue = String(value);

        if (category === 'background' && cssValue !== 'transparent') {
          vars.push(`--${shortCategory}-${key}: ${applyUserTransparency(cssValue)}`);
        } else {
          vars.push(`--${shortCategory}-${key}: ${cssValue}`);
        }

        if (category === 'border') {
          if (key === 'primary') vars.push(`--border-light: ${cssValue}`);
          if (key === 'secondary') vars.push(`--border-dark: ${cssValue}`);
        }
      });
    } else {
      vars.push(`--${category}: ${String(values)}`);
    }
  });

  if (isRecord(theme.effects)) {
    Object.entries(theme.effects).forEach(([key, value]) => {
      vars.push(`--${toKebabCase(key)}: ${String(value)}`);
    });
  }

  if (isRecord(theme.transparency)) {
    Object.entries(theme.transparency).forEach(([key, value]) => {
      vars.push(`--transparency-${toKebabCase(key)}: ${String(value)}`);
    });
  }

  if (theme.loadedAssets) {
    Object.entries(theme.loadedAssets).forEach(([assetName, assetUrl]) => {
      vars.push(`--asset-${assetName.replace(/[^a-zA-Z0-9]/g, '-')}: url('${assetUrl}')`);
    });
  }

  return vars.join(';\n      ');
}

function applyUserTransparency(value: string): string {
  if (value.includes('var(--user-transparency')) {
    return value;
  }

  const rgbaMatch = value.match(/rgba?\(([^)]+)\)/);
  if (!rgbaMatch) {
    return value;
  }

  const parts = rgbaMatch[1].split(',').map(part => part.trim());
  if (parts.length === 4) {
    return `rgba(${parts[0]}, ${parts[1]}, ${parts[2]}, calc(${parts[3]} * var(--user-transparency, 1)))`;
  }

  if (parts.length === 3) {
    return `rgba(${parts[0]}, ${parts[1]}, ${parts[2]}, var(--user-transparency, 1))`;
  }

  return value;
}
