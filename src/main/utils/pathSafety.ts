import path from 'path';

export function isPathInside(basePath: string, targetPath: string): boolean {
  const relativePath = path.relative(path.resolve(basePath), path.resolve(targetPath));
  return relativePath === '' || (!relativePath.startsWith('..') && !path.isAbsolute(relativePath));
}

export function resolveChildPath(basePath: string, ...segments: string[]): string {
  const resolvedPath = path.resolve(basePath, ...segments);
  if (!isPathInside(basePath, resolvedPath)) {
    throw new Error(`Refusing to access path outside ${basePath}`);
  }
  return resolvedPath;
}
