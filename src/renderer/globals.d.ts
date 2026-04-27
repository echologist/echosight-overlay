import type { EchosightApi } from '../shared/types';

declare global {
  interface Window {
    echosight?: EchosightApi;
  }
}

export {};
