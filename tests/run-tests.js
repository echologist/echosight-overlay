const { spawnSync } = require('node:child_process');

const groups = {
  unit: ['tests/unit/taskManager.test.js'],
  integration: ['tests/integration/ipc.test.js'],
  e2e: ['tests/e2e/criticalFlow.test.js']
};

const requestedGroups = process.argv.slice(2);
const selectedGroups = requestedGroups.length > 0 ? requestedGroups : Object.keys(groups);

const unknownGroups = selectedGroups.filter(group => !groups[group]);
if (unknownGroups.length > 0) {
  console.error(`Unknown test group(s): ${unknownGroups.join(', ')}`);
  process.exit(1);
}

const testFiles = selectedGroups.flatMap(group => groups[group]);

const result = spawnSync(process.execPath, ['--test', ...testFiles], {
  stdio: 'inherit'
});

process.exit(result.status ?? 1);
