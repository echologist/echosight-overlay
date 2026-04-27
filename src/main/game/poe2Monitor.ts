import { execFile } from 'child_process';

export interface Poe2Monitor {
  start: () => void;
  stop: () => void;
}

export interface Poe2MonitorOptions {
  onStarted: () => void;
  onStopped: () => void;
  onStillRunning?: () => void;
  intervalMs?: number;
  platform?: NodeJS.Platform;
  logger?: Pick<Console, 'log' | 'error'>;
}

const DEFAULT_INTERVAL_MS = 3000;

export function createPoe2Monitor(options: Poe2MonitorOptions): Poe2Monitor {
  const logger = options.logger || console;
  const platform = options.platform || process.platform;
  const intervalMs = options.intervalMs || DEFAULT_INTERVAL_MS;
  const processListCommand = getProcessListCommand(platform);
  let interval: ReturnType<typeof setInterval> | null = null;
  let isActive = false;
  let isRunning = false;

  const checkForPoe2 = () => {
    if (!processListCommand) {
      return;
    }

    execFile(processListCommand.command, processListCommand.args, (error, stdout) => {
      if (!isActive) {
        return;
      }

      if (error) {
        logger.error('Failed to query running processes:', error);
        return;
      }

      const wasRunning = isRunning;
      isRunning = includesPathOfExileProcess(stdout);

      if (isRunning && !wasRunning) {
        options.onStarted();
      } else if (!isRunning && wasRunning) {
        options.onStopped();
      } else if (isRunning) {
        options.onStillRunning?.();
      }
    });
  };

  return {
    start: () => {
      if (interval) {
        return;
      }

      if (!processListCommand) {
        logger.log(`PoE2 process monitoring unsupported on ${platform}`);
        return;
      }

      isActive = true;
      checkForPoe2();
      interval = setInterval(checkForPoe2, intervalMs);
    },
    stop: () => {
      isActive = false;
      isRunning = false;
      if (interval) {
        clearInterval(interval);
        interval = null;
      }
    }
  };
}

interface ProcessListCommand {
  command: string;
  args: string[];
}

function getProcessListCommand(platform: NodeJS.Platform): ProcessListCommand | null {
  switch (platform) {
    case 'win32':
      return { command: 'tasklist', args: ['/fo', 'csv'] };
    case 'darwin':
    case 'linux':
      return { command: 'ps', args: ['-A', '-o', 'comm=', '-o', 'args='] };
    default:
      return null;
  }
}

function includesPathOfExileProcess(processListOutput: string): boolean {
  const normalized = processListOutput.toLowerCase().replace(/[^a-z0-9]/g, '');
  return normalized.includes('pathofexile');
}
