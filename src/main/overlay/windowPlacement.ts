export interface WorkArea {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface OverlayWindowBounds {
  width: number;
  height: number;
  x: number;
  y: number;
}

export interface OverlayPlacementOptions {
  width?: number;
  height?: number;
  margin?: number;
}

const DEFAULT_OVERLAY_WIDTH = 500;
const DEFAULT_OVERLAY_HEIGHT = 700;
const DEFAULT_MARGIN = 20;

export function getOverlayWindowBounds(
  workArea: WorkArea,
  options: OverlayPlacementOptions = {}
): OverlayWindowBounds {
  const width = options.width ?? DEFAULT_OVERLAY_WIDTH;
  const height = options.height ?? DEFAULT_OVERLAY_HEIGHT;
  const [x, y] = getOverlayAnchorPosition(workArea, {
    width,
    margin: options.margin
  });

  return {
    width,
    height,
    x,
    y
  };
}

export function getOverlayAnchorPosition(
  workArea: WorkArea,
  options: Pick<OverlayPlacementOptions, 'width' | 'margin'> = {}
): [number, number] {
  const width = options.width ?? DEFAULT_OVERLAY_WIDTH;
  const margin = options.margin ?? DEFAULT_MARGIN;
  const x = workArea.x + Math.max(0, workArea.width - width - margin);
  const y = workArea.y + margin;

  return [x, y];
}
