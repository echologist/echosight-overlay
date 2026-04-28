import { describe, expect, test } from 'vitest';
import {
  getOverlayAnchorPosition,
  getOverlayWindowBounds
} from '../../../src/main/overlay/windowPlacement';

describe('overlay window placement', () => {
  test('anchors to the right side of the primary work area', () => {
    expect(getOverlayWindowBounds({
      x: 0,
      y: 0,
      width: 1920,
      height: 1040
    })).toEqual({
      width: 500,
      height: 700,
      x: 1400,
      y: 20
    });
  });

  test('preserves non-zero work area origins on secondary or inset displays', () => {
    expect(getOverlayAnchorPosition({
      x: -1920,
      y: 40,
      width: 1920,
      height: 1000
    }, {
      width: 500
    })).toEqual([-520, 60]);
  });

  test('does not place the overlay outside a narrow work area', () => {
    expect(getOverlayAnchorPosition({
      x: 100,
      y: 30,
      width: 420,
      height: 700
    }, {
      width: 500
    })).toEqual([100, 50]);
  });
});
