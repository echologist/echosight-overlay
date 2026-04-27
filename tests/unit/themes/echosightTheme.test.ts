import { readFile } from 'fs/promises';
import path from 'path';
import { describe, expect, test } from 'vitest';
import type { Theme } from '../../../src/shared/types';
import { generateThemeCSS } from '../../../src/renderer/features/themes/themeCss';

describe('Echosight theme', () => {
  test('uses the user transparency directly in click-through mode', async () => {
    const theme = JSON.parse(
      await readFile(path.join(process.cwd(), 'data', 'themes', 'echosight.json'), 'utf8')
    ) as Theme;
    const css = generateThemeCSS(theme, 0.7);
    const transparency = theme.transparency as { clickThrough?: unknown } | undefined;

    expect(theme.version).toBe('1.0.3');
    expect(transparency?.clickThrough).toBe('var(--user-transparency, 0.7)');
    expect(css).toContain('--user-transparency: 0.7');
    expect(css).toContain('rgba(13,11,24, var(--user-transparency, 0.7))');
    expect(css).not.toContain('* 1.6');
  });
});
