import { resolve } from 'path'
import { defineConfig, externalizeDepsPlugin } from 'electron-vite'

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
    build: {
      outDir: 'dist-electron/main'
    }
  },
  preload: {
    plugins: [externalizeDepsPlugin()], 
    build: {
      outDir: 'dist-electron/preload'
    }
  },
  renderer: {
    build: {
      outDir: 'dist-electron/renderer'
    }
  }
})