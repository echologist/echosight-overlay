import { defineConfig } from 'vitest/config';
export default defineConfig({
  test: {
    environment: 'node',
    include: [
      'tools/golden/theme-library.gen.test.ts',
      'tests/unit/main/themeLibrary.test.ts',
      'tests/unit/main/themeAssetLoaders.test.ts',
      'tests/unit/themes/themeSounds.test.ts'
    ]
  }
});
