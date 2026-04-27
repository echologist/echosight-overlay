import type { Theme } from '../../../shared/types';
import {
  generateClickThroughStyles,
  generateCommonStyles,
  generateCustomCSS,
  generateInteractiveStyles
} from './themeCssBaseStyles';
import { generateAnimationStyles } from './themeCssAnimation';
import { generateBackgroundStyles } from './themeCssBackground';
import { generateComponentStyles } from './themeCssComponents';
import type { ThemeForCss } from './themeCssHelpers';
import { generateLayoutStyles } from './themeCssLayout';
import { generateCSSVariables } from './themeCssVariables';

export function generateThemeCSS(theme: Theme, transparency: number, cssFileStyles = ''): string {
  const themeConfig = theme as ThemeForCss;
  const cssVars = generateCSSVariables(themeConfig, transparency);
  const interactiveStyles = generateInteractiveStyles(themeConfig);
  const clickThroughStyles = generateClickThroughStyles(themeConfig);
  const commonStyles = generateCommonStyles(themeConfig);
  const componentStyles = generateComponentStyles(themeConfig);
  const customCSS = generateCustomCSS(themeConfig);
  const backgroundStyles = generateBackgroundStyles(themeConfig);
  const layoutStyles = generateLayoutStyles(themeConfig);
  const animationStyles = generateAnimationStyles(themeConfig);

  return `
    :root {
      ${cssVars}
    }

    ${backgroundStyles}
    ${layoutStyles}
    ${animationStyles}
    ${commonStyles}
    ${componentStyles}
    ${interactiveStyles}
    ${clickThroughStyles}
    ${customCSS}

    /* CSS File Styles */
    ${cssFileStyles}
  `;
}
