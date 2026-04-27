import { promises as fs } from 'fs';
import path from 'path';
import { destr } from 'destr';

export async function readJsonFile<T = unknown>(filePath: string): Promise<T> {
  const data = await fs.readFile(filePath, 'utf8');
  return destr<T>(data, { strict: true });
}

export async function writeJsonFile(filePath: string, value: unknown): Promise<void> {
  await fs.mkdir(path.dirname(filePath), { recursive: true });
  await fs.writeFile(filePath, JSON.stringify(value, null, 2));
}
