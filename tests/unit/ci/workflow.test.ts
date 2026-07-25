import {
  chmodSync,
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
  shell?: string;
  with?: Record<string, boolean | string>;
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

function runTagValidation(httpStatus: string, curlExit = 0, tagName = 'v2.0.0') {
  const directory = mkdtempSync(join(tmpdir(), 'echosight-tag-test-'));
  const bin = join(directory, 'bin');
  mkdirSync(bin);
  const curl = join(bin, 'curl');
  writeFileSync(
    curl,
    `#!/usr/bin/env bash
set -u
output=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    -o|--output) output="$2"; shift 2 ;;
    -w|--write-out) shift 2 ;;
    *) shift ;;
  esac
done
if [ "$FAKE_CURL_EXIT" -ne 0 ]; then
  exit "$FAKE_CURL_EXIT"
fi
printf '{"status":"fake"}' > "$output"
printf '%s' "$FAKE_HTTP_STATUS"
`
  );
  chmodSync(curl, 0o755);
  const gh = join(bin, 'gh');
  writeFileSync(gh, '#!/usr/bin/env bash\nexit 1\n');
  chmodSync(gh, 0o755);

  const validation = step(workflow.jobs.release, 'Verify tag matches package version')?.run ?? '';
  const result = spawnSync('bash', ['-c', `set -euo pipefail\n${validation}`], {
    encoding: 'utf8',
    env: {
      ...process.env,
      FAKE_CURL_EXIT: String(curlExit),
      FAKE_HTTP_STATUS: httpStatus,
      GH_TOKEN: 'test-token',
      GITHUB_API_URL: 'https://api.github.test',
      GITHUB_REPOSITORY: 'echologist/echoesight-overlay',
      PACKAGE_VERSION: '2.0.0',
      PATH: `${bin}:${process.env.PATH}`,
      TAG_NAME: tagName
    }
  });
  rmSync(directory, { recursive: true, force: true });

  return { status: result.status, output: `${result.stdout}${result.stderr}` };
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
    expect(tagCheck?.run).toContain(
      '$GITHUB_API_URL/repos/$GITHUB_REPOSITORY/releases/tags/$TAG'
    );
  });

  test('fails closed when checking whether a release exists', () => {
    expect(runTagValidation('404').status).toBe(0);

    for (const [name, result] of [
      ['existing release', runTagValidation('200')],
      ['server error', runTagValidation('500')],
      ['transport failure', runTagValidation('000', 7)],
      ['tag mismatch', runTagValidation('404', 0, 'v2.0.1')]
    ] as const) {
      expect.soft(result.status, `${name}: ${result.output}`).not.toBe(0);
    }
  });

  test('keeps GitHub expressions out of shell scripts', () => {
    for (const [jobName, job] of Object.entries(workflow.jobs)) {
      for (const candidate of job.steps ?? []) {
        expect(candidate.run ?? '', `${jobName}: ${candidate.name}`).not.toContain('${{');
      }
    }

    const build = step(workflow.jobs.build, 'Build application');
    expect(workflow.jobs.build.strategy?.matrix?.include).toEqual(
      expect.arrayContaining([expect.objectContaining({ os: 'windows-latest' })])
    );
    expect(build?.shell).toBe('bash');
    expect(build?.env?.BUILD_SCRIPT).toBe('${{ matrix.build_script }}');
    expect(build?.run).toBe('npm run "$BUILD_SCRIPT"');
  });

  test('does not persist the write token during release checkout', () => {
    const checkout = step(workflow.jobs.release, 'Checkout code');
    expect(checkout?.with?.['persist-credentials']).toBe(false);
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
    const rootRequire = createRequire(import.meta.url);
    const expectedVersions = {
      '@electron/asar': '1.1.16',
      '@electron/universal': '2.1.2',
      'dir-compare': '1.1.16',
      filelist: '2.1.2',
      glob: '1.1.16'
    };
    const packageJson = JSON.parse(readFileSync(join(process.cwd(), 'package.json'), 'utf8')) as {
      overrides?: Record<string, string>;
    };
    const lockfile = JSON.parse(readFileSync(join(process.cwd(), 'package-lock.json'), 'utf8')) as {
      packages: Record<string, { version?: string }>;
    };

    expect(packageJson.overrides).toEqual({
      'brace-expansion@^1.0.0': '1.1.16',
      'brace-expansion@^2.0.0': '2.1.2',
      'brace-expansion@^5.0.0': '5.0.8'
    });

    for (const [owner, version] of Object.entries(expectedVersions)) {
      const ownerRequire = createRequire(rootRequire.resolve(owner));
      const loaded = ownerRequire('minimatch') as
        | ((value: string, pattern: string) => boolean)
        | {
            default?: (value: string, pattern: string) => boolean;
            minimatch?: (value: string, pattern: string) => boolean;
          };
      const matcher = typeof loaded === 'function' ? loaded : loaded.minimatch ?? loaded.default;

      expect(
        (ownerRequire('brace-expansion/package.json') as { version: string }).version,
        `${owner} installed brace-expansion`
      ).toBe(version);
      expect(
        lockfile.packages[`node_modules/${owner}/node_modules/brace-expansion`]?.version,
        `${owner} locked brace-expansion`
      ).toBe(version);
      expect(matcher, `${owner} minimatch export`).toBeTypeOf('function');
      expect(matcher?.('a/b', 'a/{b,c}'), owner).toBe(true);
      expect(matcher?.('a/02', 'a/{01..03}'), `${owner} padded range`).toBe(true);
    }

    const rootMinimatchRequire = createRequire(rootRequire.resolve('minimatch'));
    expect(
      (rootMinimatchRequire('brace-expansion/package.json') as { version: string }).version
    ).toBe('5.0.8');
    expect(lockfile.packages['node_modules/brace-expansion']?.version).toBe('5.0.8');
  });

  test('uses only the official npm registry in the lockfile', () => {
    const lockfile = readFileSync(join(process.cwd(), 'package-lock.json'), 'utf8');
    expect(lockfile).not.toContain('registry.npmmirror.com');
  });

  test('never generates placeholder icons', () => {
    expect(workflowSource).not.toMatch(/placeholder icon|Create missing icon/i);
  });
});
