import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    environment: 'jsdom',
    include: [
      'tools/golden/settings.gen.test.ts',
      'tests/unit/settings/settingsDomain.test.ts',
      'tests/unit/settings/hotkeyRecorder.test.ts',
      'tests/unit/settings/settingsUi.test.ts',
      'tests/unit/main/hotkeys.test.ts'
    ],
    restoreMocks: true
  }
});
