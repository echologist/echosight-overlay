import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: [
      'tools/golden/themes.gen.test.ts',
      'tests/unit/main/themeValidation.test.ts',
      'tests/unit/themes/themeHelpers.test.ts'
    ]
  }
});
