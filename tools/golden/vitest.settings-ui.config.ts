import { defineConfig } from 'vitest/config';
// Isolated config: only the settings/hotkey oracle, jsdom, serial. Electron is
// stubbed inside the test via vi.mock('electron') (dependency stub only).
export default defineConfig({ test: {
  environment: 'jsdom', include: ['tools/golden/settings-ui.test.ts'], fileParallelism: false
}});
