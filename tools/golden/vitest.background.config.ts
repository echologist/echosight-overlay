import { defineConfig } from 'vitest/config';
export default defineConfig({
  test: {
    environment: 'jsdom',
    include: ['tools/golden/background.gen.test.ts', 'tools/golden/background-ui.test.ts', 'tests/unit/tasks/triggerController.test.ts'],
    fileParallelism: false
  }
});
