interface SingleInstanceApp {
  requestSingleInstanceLock: () => boolean;
  quit: () => void;
  on: (event: 'second-instance', listener: () => void) => unknown;
}

export function ensureSingleInstance(
  app: SingleInstanceApp,
  onSecondInstance: () => void
): boolean {
  if (!app.requestSingleInstanceLock()) {
    app.quit();
    return false;
  }

  app.on('second-instance', onSecondInstance);
  return true;
}
