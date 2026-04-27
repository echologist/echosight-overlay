import type {
  AlertHandler,
  ConfirmHandler
} from './dialogTypes';

export interface OverlayWindowCommandApi {
  minimizeOverlay: () => void;
  quitApplication: () => void;
  resetWindowPosition: () => void;
  toggleInteractiveMode: () => void;
}

type LogSink = Pick<Console, 'log' | 'error'>;

export function resetOverlayPosition(
  api: OverlayWindowCommandApi,
  alertUser: AlertHandler,
  logger: LogSink = console
): void {
  try {
    api.resetWindowPosition();
    void alertUser('Window position reset! The overlay will move to the top-right corner.');
  } catch (error) {
    logger.error('Error resetting overlay position:', error);
  }
}

export function minimizeOverlayWindow(api: OverlayWindowCommandApi, logger: LogSink = console): void {
  try {
    logger.log('minimizeOverlay called');
    api.minimizeOverlay();
  } catch (error) {
    logger.error('Error minimizing overlay:', error);
  }
}

export async function quitOverlayApplication(
  api: OverlayWindowCommandApi,
  confirmUser: ConfirmHandler,
  logger: LogSink = console
): Promise<void> {
  try {
    logger.log('closeOverlay called - quitting application');
    if (await confirmUser('Close Echosight Overlay completely?', {
      title: 'Quit Echosight',
      tone: 'danger'
    })) {
      api.quitApplication();
    }
  } catch (error) {
    logger.error('Error closing overlay:', error);
  }
}

export function toggleOverlayInteractiveMode(api: OverlayWindowCommandApi, logger: LogSink = console): void {
  try {
    logger.log('toggleInteractiveMode called');
    api.toggleInteractiveMode();
  } catch (error) {
    logger.error('Error toggling interactive mode:', error);
  }
}
