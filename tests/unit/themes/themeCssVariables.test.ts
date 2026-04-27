import { describe, expect, test } from 'vitest';
import { generateCSSVariables } from '../../../src/renderer/features/themes/themeCssVariables';
import type { ThemeForCss } from '../../../src/renderer/features/themes/themeCssHelpers';

describe('theme CSS variables', () => {
  test('preserves background colors that already use user transparency', () => {
    const css = generateCSSVariables({
      id: 'transparent-theme',
      name: 'Transparent Theme',
      colors: {
        background: {
          primary: 'rgba(13, 11, 24, var(--user-transparency, 0.95))'
        }
      }
    } as ThemeForCss, 0.6);

    expect(css).toContain('--user-transparency: 0.6');
    expect(css).toContain('--bg-primary: rgba(13, 11, 24, var(--user-transparency, 0.95))');
    expect(css).not.toContain('calc(var(--user-transparency');
  });

  test('adds user transparency to fixed rgba background colors', () => {
    const css = generateCSSVariables({
      id: 'fixed-theme',
      name: 'Fixed Theme',
      colors: {
        background: {
          primary: 'rgba(13, 11, 24, 0.95)'
        }
      }
    } as ThemeForCss, 0.6);

    expect(css).toContain('--bg-primary: rgba(13, 11, 24, calc(0.95 * var(--user-transparency, 1)))');
  });
});
