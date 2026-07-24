import { promises as fs } from 'fs';
import os from 'os';
import path from 'path';
import { afterEach, beforeEach, describe, expect, test, vi } from 'vitest';
import { writeJsonFile } from '../../../src/main/utils/jsonStorage';

describe('JSON storage writes', () => {
  let tempRoot = '';
  let targetPath = '';

  beforeEach(async () => {
    tempRoot = await fs.mkdtemp(path.join(os.tmpdir(), 'echosight-json-storage-'));
    targetPath = path.join(tempRoot, 'tasks.json');
  });

  afterEach(async () => {
    await fs.rm(tempRoot, { recursive: true, force: true });
  });

  test('serializes concurrent writes to the same resolved path in caller order', async () => {
    const originalMkdir = fs.mkdir.bind(fs);
    let mkdirCalls = 0;
    let releaseFirstMkdir!: () => void;
    let markFirstMkdirStarted!: () => void;
    const firstMkdirGate = new Promise<void>(resolve => {
      releaseFirstMkdir = resolve;
    });
    const firstMkdirStarted = new Promise<void>(resolve => {
      markFirstMkdirStarted = resolve;
    });

    vi.spyOn(fs, 'mkdir').mockImplementation(async (...args) => {
      mkdirCalls++;
      if (mkdirCalls === 1) {
        markFirstMkdirStarted();
        await firstMkdirGate;
      }
      return originalMkdir(...args);
    });

    const firstWrite = writeJsonFile(targetPath, { caller: 1 });
    await firstMkdirStarted;
    const secondWrite = writeJsonFile(path.relative(process.cwd(), targetPath), { caller: 2 });
    await Promise.resolve();

    expect(mkdirCalls).toBe(1);

    releaseFirstMkdir();
    await Promise.all([firstWrite, secondWrite]);

    await expect(readJson(targetPath)).resolves.toEqual({ caller: 2 });
  });

  test('failed atomic replacement preserves destination and removes temporary file', async () => {
    await fs.writeFile(targetPath, 'old bytes', 'utf8');
    vi.spyOn(fs, 'rename').mockRejectedValueOnce(new Error('rename failed'));

    await expect(writeJsonFile(targetPath, { caller: 1 })).rejects.toThrow('rename failed');
    await expect(fs.readFile(targetPath, 'utf8')).resolves.toBe('old bytes');
    await expect(fs.readdir(tempRoot)).resolves.toEqual(['tasks.json']);
  });

  test('write queued before a rejected write still succeeds', async () => {
    await fs.writeFile(targetPath, 'old bytes', 'utf8');
    let rejectFirstRename!: () => void;
    let markFirstRenameStarted!: () => void;
    const firstRenameGate = new Promise<void>(resolve => {
      rejectFirstRename = resolve;
    });
    const firstRenameStarted = new Promise<void>(resolve => {
      markFirstRenameStarted = resolve;
    });
    vi.spyOn(fs, 'rename').mockImplementationOnce(async () => {
      markFirstRenameStarted();
      await firstRenameGate;
      throw new Error('rename failed');
    });

    const firstWrite = writeJsonFile(targetPath, { caller: 1 });
    const firstResult = expect(firstWrite).rejects.toThrow('rename failed');
    await firstRenameStarted;
    const secondWrite = writeJsonFile(targetPath, { caller: 2 });

    rejectFirstRename();
    await firstResult;
    await expect(secondWrite).resolves.toBeUndefined();
    await expect(readJson(targetPath)).resolves.toEqual({ caller: 2 });
  });

  test('cleanup failure does not replace the decisive write error', async () => {
    await fs.writeFile(targetPath, 'old bytes', 'utf8');
    vi.spyOn(fs, 'rename').mockRejectedValueOnce(new Error('rename failed'));
    vi.spyOn(fs, 'rm').mockRejectedValueOnce(new Error('cleanup failed'));

    await expect(writeJsonFile(targetPath, { caller: 1 })).rejects.toThrow('rename failed');
  });

  test('flush failure rejects and preserves destination', async () => {
    await fs.writeFile(targetPath, 'old bytes', 'utf8');
    const originalOpen = fs.open.bind(fs);

    vi.spyOn(fs, 'open').mockImplementationOnce(async (...args) => {
      const handle = await originalOpen(...args);
      vi.spyOn(handle, 'sync').mockRejectedValueOnce(new Error('flush failed'));
      return handle;
    });

    await expect(writeJsonFile(targetPath, { caller: 1 })).rejects.toThrow('flush failed');
    await expect(fs.readFile(targetPath, 'utf8')).resolves.toBe('old bytes');
    await expect(fs.readdir(tempRoot)).resolves.toEqual(['tasks.json']);
  });
});

async function readJson(filePath: string): Promise<unknown> {
  return JSON.parse(await fs.readFile(filePath, 'utf8')) as unknown;
}
