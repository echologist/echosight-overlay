import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'node',
    include: [
      'tools/golden/tasks.gen.test.ts',
      'tests/unit/tasks/taskStateController.test.ts'
    ],
    fileParallelism: false
  }
});
