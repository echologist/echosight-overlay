import { defineConfig } from 'vitest/config';

// Deliberately independent of the UI/jsdom suite; no Electron runtime needed.
export default defineConfig({
  test: {
    environment: 'node',
    include: [
      'tools/golden/storage.gen.test.ts',
      'tests/unit/main/jsonStorage.test.ts',
      'tests/unit/main/storageIpc.test.ts',
      'tests/unit/main/runtimeDataBootstrap.test.ts'
    ],
    restoreMocks: true,
    fileParallelism: false
  }
});
