import { defineConfig } from 'vitest/config';
export default defineConfig({ test: {
  environment: 'jsdom', include: ['tools/golden/template-ui.test.ts'], fileParallelism: false
}});
