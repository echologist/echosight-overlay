#!/usr/bin/env node

const { spawnSync } = require('node:child_process');
const { readFileSync } = require('node:fs');

const args = process.argv.slice(2);
const shouldPush = args.includes('--push');
const isDryRun = args.includes('--dry-run');
const versionArg = args.find(arg => !arg.startsWith('--'));

function usage(exitCode = 0) {
  const output = exitCode === 0 ? console.log : console.error;
  output(`Usage: npm run release -- <version|patch|minor|major> [--push] [--dry-run]

Examples:
  npm run release -- patch
  npm run release -- 1.2.1
  npm run release -- minor --push

By default this bumps package.json/package-lock.json, creates a release commit,
and creates a matching vX.Y.Z tag locally. Add --push to push main and the tag.`);
  process.exit(exitCode);
}

function run(command, args, options = {}) {
  const printable = [command, ...args].join(' ');
  if (isDryRun) {
    console.log(`[dry-run] ${printable}`);
    return '';
  }

  const result = spawnSync(command, args, {
    encoding: 'utf8',
    stdio: options.capture ? ['ignore', 'pipe', 'pipe'] : 'inherit'
  });

  if (result.status !== 0) {
    if (options.capture && result.stderr) {
      process.stderr.write(result.stderr);
    }
    process.exit(result.status || 1);
  }

  return options.capture ? result.stdout.trim() : '';
}

function assertNoUnknownOptions() {
  const allowedOptions = new Set(['--push', '--dry-run', '--help', '-h']);
  const unknownOption = args.find(arg => arg.startsWith('--') && !allowedOptions.has(arg));
  if (unknownOption) {
    console.error(`Unknown option: ${unknownOption}`);
    usage(1);
  }
}

function assertCleanTrackedWorktree() {
  run('git', ['diff', '--quiet']);
  run('git', ['diff', '--cached', '--quiet']);
}

function assertTagDoesNotExist(tag) {
  if (isDryRun) {
    console.log(`[dry-run] verify tag does not exist: ${tag}`);
    return;
  }

  const result = spawnSync('git', ['rev-parse', '--verify', '--quiet', `refs/tags/${tag}`], {
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe']
  });

  if (result.status === 0) {
    console.error(`Tag ${tag} already exists.`);
    process.exit(1);
  }
}

function readPackageVersion() {
  return JSON.parse(readFileSync('package.json', 'utf8')).version;
}

assertNoUnknownOptions();

if (args.includes('--help') || args.includes('-h')) {
  usage(0);
}

if (!versionArg) {
  usage(1);
}

assertCleanTrackedWorktree();

run('npm', ['version', versionArg, '--no-git-tag-version']);

const version = isDryRun ? `<next ${versionArg}>` : readPackageVersion();
const tag = `v${version}`;

assertTagDoesNotExist(tag);

run('git', ['add', 'package.json', 'package-lock.json']);
run('git', ['commit', '-m', `chore: release ${tag}`]);
run('git', ['tag', tag]);

if (shouldPush) {
  run('git', ['push', 'origin', 'main']);
  run('git', ['push', 'origin', tag]);
} else {
  console.log(`Prepared ${tag} locally.`);
  console.log(`Push it with: git push origin main && git push origin ${tag}`);
}
