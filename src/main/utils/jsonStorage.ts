import { promises as fs } from 'fs';
import { randomUUID } from 'crypto';
import path from 'path';
import { destr } from 'destr';

const pendingWrites = new Map<string, Promise<void>>();

export function parseJson<T = unknown>(data: string): T {
  return destr<T>(data, { strict: true });
}

export async function readJsonFile<T = unknown>(filePath: string): Promise<T> {
  const data = await fs.readFile(filePath, 'utf8');
  return parseJson<T>(data);
}

export async function writeJsonFile(filePath: string, value: unknown): Promise<void> {
  const targetPath = path.resolve(filePath);
  const write = (pendingWrites.get(targetPath) || Promise.resolve())
    .catch(() => undefined)
    .then(() => replaceJsonFile(targetPath, value));

  pendingWrites.set(targetPath, write);

  try {
    await write;
  } finally {
    if (pendingWrites.get(targetPath) === write) {
      pendingWrites.delete(targetPath);
    }
  }
}

async function replaceJsonFile(targetPath: string, value: unknown): Promise<void> {
  const directory = path.dirname(targetPath);
  const tempPath = path.join(
    directory,
    `.${path.basename(targetPath)}.${process.pid}.${randomUUID()}.tmp`
  );
  let handle: Awaited<ReturnType<typeof fs.open>> | undefined;

  await fs.mkdir(directory, { recursive: true });

  try {
    handle = await fs.open(tempPath, 'wx');
    await handle.writeFile(JSON.stringify(value, null, 2), 'utf8');
    await handle.sync();
    await handle.close();
    handle = undefined;
    await fs.rename(tempPath, targetPath);
  } catch (error) {
    if (handle) {
      try {
        await handle.close();
      } catch {
        // Preserve the original write failure.
      }
    }
    try {
      await fs.rm(tempPath, { force: true });
    } catch {
      // Preserve the original write failure.
    }
    throw error;
  }
}
