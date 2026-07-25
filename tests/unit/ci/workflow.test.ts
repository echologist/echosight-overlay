import {
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync
} from 'node:fs';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import { describe, expect, test } from 'vitest';
import { parse } from 'yaml';

type Step = {
  'continue-on-error'?: boolean;
  env?: Record<string, string>;
  name?: string;
  uses?: string;
  run?: string;
  with?: Record<string, string>;
};

type Job = {
  'continue-on-error'?: boolean;
  needs?: string | string[];
  if?: string;
  permissions?: Record<string, string>;
  strategy?: {
    matrix?: { include?: Array<{ artifacts?: string; build_script?: string; os?: string }> };
  };
  steps?: Step[];
};

type Workflow = {
  on: {
    pull_request?: unknown;
    push?: { branches?: string[]; tags?: string[] };
    workflow_dispatch?: unknown;
  };
  permissions?: Record<string, string>;
  jobs: Record<string, Job>;
};

const workflowSource = readFile();
const workflow = parse(workflowSource) as Workflow;

function readFile(): string {
  return readFileSync(join(process.cwd(), '.github/workflows/build.yml'), 'utf8');
}

function scripts(job?: Job): string {
  return job?.steps?.map((step) => step.run ?? '').join('\n') ?? '';
}

function needs(job: Job): string[] {
  return Array.isArray(job.needs) ? job.needs : job.needs ? [job.needs] : [];
}

function expression(condition?: string): string {
  return condition?.replace(/^\s*\${{\s*/, '').replace(/\s*}}\s*$/, '') ?? '';
}

function step(job: Job, name: string): Step | undefined {
  return job.steps?.find((candidate) => candidate.name === name);
}

function runReleasePreparation(files: string[]) {
  const directory = mkdtempSync(join(tmpdir(), 'echosight-release-test-'));
  const artifacts = join(directory, 'artifacts', 'build');
  mkdirSync(artifacts, { recursive: true });
  files.forEach((file) => writeFileSync(join(artifacts, file), 'test'));

  const releasePreparation = step(workflow.jobs.release, 'Prepare release assets')?.run ?? '';
  const result = spawnSync('bash', ['-c', `set -euo pipefail\n${releasePreparation}`], {
    cwd: directory,
    encoding: 'utf8'
  });
  const releaseDirectory = join(directory, 'release-assets');
  const released = existsSync(releaseDirectory) ? readdirSync(releaseDirectory).sort() : [];
  rmSync(directory, { recursive: true, force: true });

  return { status: result.status, output: `${result.stdout}${result.stderr}`, released };
}

describe('release workflow', () => {
  test('runs verification for pull requests, main pushes, tags, and manual dispatches', () => {
    expect(workflow.on).toHaveProperty('pull_request');
    expect(workflow.on).toHaveProperty('workflow_dispatch');
    expect(workflow.on.push?.branches).toContain('main');
    expect(workflow.on.push?.tags).toContain('v*');
    expect(workflow.jobs['lint-workflows'].if).toBeUndefined();
    expect(workflow.jobs.verify.if).toBeUndefined();
  });

  test('grants write permission only to the release job', () => {
    expect(workflow.permissions?.contents).toBe('read');
    expect(workflow.jobs.release.permissions?.contents).toBe('write');

    for (const [name, job] of Object.entries(workflow.jobs)) {
      if (name !== 'release') expect(job.permissions?.contents).not.toBe('write');
    }
  });

  test('verifies source and a Linux unpacked package before platform builds', () => {
    const verifyScripts = scripts(workflow.jobs.verify);
    expect(verifyScripts).toContain('npm ci');
    expect(verifyScripts).toContain('npm run verify');
    expect(verifyScripts).toContain('electron-builder --linux --dir --publish=never');
  });

  test('builds only manual runs and v-tag pushes after lint, verification, and audit', () => {
    expect(needs(workflow.jobs.build)).toEqual(
      expect.arrayContaining(['lint-workflows', 'verify', 'audit-build-chain'])
    );
    expect(expression(workflow.jobs.build.if)).toBe(
      "github.event_name == 'workflow_dispatch' || (github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v'))"
    );
  });

  test('blocks packaging when the build dependency audit fails', () => {
    const audit = workflow.jobs['audit-build-chain'];

    expect(expression(audit.if)).toBe(
      "github.event_name == 'workflow_dispatch' || (github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v'))"
    );
    expect(step(audit, 'Audit build dependencies')?.run).toBe('npm audit --audit-level=high');

    for (const job of Object.values(workflow.jobs)) {
      expect(job['continue-on-error']).not.toBe(true);
      for (const candidate of job.steps ?? []) {
        expect(candidate['continue-on-error']).not.toBe(true);
      }
    }
  });

  test('releases only validated, new v-tag pushes after build', () => {
    const releaseScripts = scripts(workflow.jobs.release);
    expect(needs(workflow.jobs.release)).toEqual(['build']);
    expect(expression(workflow.jobs.release.if)).toBe(
      "github.event_name == 'push' && startsWith(github.ref, 'refs/tags/v')"
    );
    expect(workflow.jobs.release.if).not.toContain('always()');
    expect(releaseScripts).toContain("require('./package.json').version");
    const tagCheck = step(workflow.jobs.release, 'Verify tag matches package version');
    expect(tagCheck?.env?.TAG_NAME).toContain('github.ref_name');
    expect(tagCheck?.run).not.toContain('${{');
    expect(releaseScripts).toContain('gh release view');
  });

  test('pins every external action to a commit', () => {
    const references = Object.values(workflow.jobs).flatMap(
      (job) => job.steps?.flatMap((candidate) => candidate.uses ?? []) ?? []
    );
    expect(references.length).toBeGreaterThan(0);
    expect(references.filter((reference) => !/@[0-9a-f]{40}$/.test(reference))).toEqual([]);
  });

  test('downloads and verifies actionlint before extraction and execution', () => {
    const lintJob = workflow.jobs['lint-workflows'];
    const downloadIndex = lintJob.steps?.findIndex(({ name }) => name === 'Download actionlint') ?? -1;
    const checkIndex = lintJob.steps?.findIndex(({ name }) => name === 'Check workflow files') ?? -1;
    const download = step(lintJob, 'Download actionlint')?.run ?? '';
    const allScripts = Object.values(workflow.jobs).map(scripts).join('\n');

    expect(allScripts).not.toMatch(
      /(?:curl|wget)[^\n]*\|\s*(?:ba)?sh|(?:ba)?sh\s*<\(\s*(?:curl|wget)/
    );
    expect(download).toContain('actionlint_${ACTIONLINT_VERSION}_linux_amd64.tar.gz');
    expect(download).not.toContain('x86_64');
    expect(download).toContain('8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8');
    expect(download.indexOf('sha256sum --check')).toBeGreaterThan(-1);
    expect(download.indexOf('tar ')).toBeGreaterThan(download.indexOf('sha256sum --check'));
    expect(checkIndex).toBeGreaterThan(downloadIndex);
    expect(step(lintJob, 'Check workflow files')?.run).toContain('/tmp/actionlint');
  });

  test('fails uploads without every platform artifact glob', () => {
    const upload = step(workflow.jobs.build, 'Upload artifacts');
    const matrix = workflow.jobs.build.strategy?.matrix?.include;

    expect(upload?.with?.['if-no-files-found']).toBe('error');
    expect(matrix).toContainEqual(
      expect.objectContaining({ os: 'windows-latest', artifacts: 'dist/*.exe' })
    );
    expect(matrix).toContainEqual(
      expect.objectContaining({ os: 'macos-latest', artifacts: 'dist/*.dmg' })
    );
    expect(matrix).toContainEqual(
      expect.objectContaining({
        os: 'ubuntu-latest',
        artifacts: expect.stringContaining('dist/*.AppImage')
      })
    );
    expect(matrix?.find(({ os }) => os === 'ubuntu-latest')?.artifacts).toContain('dist/*.deb');
  });

  test('requires exact release asset counts and both macOS architectures', () => {
    const valid = [
      'Echosight.exe',
      'Echosight.dmg',
      'Echosight-arm64.dmg',
      'Echosight.AppImage',
      'echosight_amd64.deb'
    ];
    const success = runReleasePreparation(valid);
    expect(success.status).toBe(0);
    expect(success.released).toEqual([...valid].sort());

    const invalidSets = [
      valid.filter((file) => !file.endsWith('.exe')),
      valid.filter((file) => !file.endsWith('.dmg')),
      [...valid, 'Echosight-extra.dmg'],
      valid.map((file) => (file.includes('arm64') ? 'Echosight-second-x64.dmg' : file)),
      [...valid, 'Echosight-second.AppImage'],
      valid.filter((file) => !file.endsWith('.deb'))
    ];

    for (const invalid of invalidSets) {
      const failure = runReleasePreparation(invalid);
      expect(failure.status, failure.output).not.toBe(0);
      expect(failure.output).toContain('::error::');
    }
  });

  test('keeps brace expansion compatible across packaging dependency generations', () => {
    const require = createRequire(import.meta.url);
    const owners = ['@electron/asar', '@electron/universal', 'dir-compare', 'filelist', 'glob'];

    for (const owner of owners) {
      const ownerRequire = createRequire(require.resolve(owner));
      const loaded = ownerRequire('minimatch') as
        | ((value: string, pattern: string) => boolean)
        | {
            default?: (value: string, pattern: string) => boolean;
            minimatch?: (value: string, pattern: string) => boolean;
          };
      const matcher = typeof loaded === 'function' ? loaded : loaded.minimatch ?? loaded.default;

      expect(matcher, `${owner} minimatch export`).toBeTypeOf('function');
      expect(matcher?.('a/b', 'a/{b,c}'), owner).toBe(true);
      expect(matcher?.('a/02', 'a/{01..03}'), `${owner} padded range`).toBe(true);
    }
  });

  test('never generates placeholder icons', () => {
    expect(workflowSource).not.toMatch(/placeholder icon|Create missing icon/i);
  });
});
