import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          if (!id.includes('node_modules')) return
          if (id.includes('@react-pdf/textkit')) return 'react-pdf-textkit'
          if (id.includes('@react-pdf/layout')) return 'react-pdf-layout'
          if (id.includes('@react-pdf/render')) return 'react-pdf-render'
          if (id.includes('@react-pdf/renderer')) return 'react-pdf-renderer'
          if (id.includes('@react-pdf/png-js')) return 'react-pdf-png'
          if (id.includes('@react-pdf/reconciler')) return 'pdf-reconciler'
          if (id.includes('@react-pdf/')) return 'pdf-core'
          if (id.includes('fontkit')) return 'fontkit'
          if (id.includes('yoga-layout')) return 'yoga'
          if (id.includes('crypto-js')) return 'crypto-js'
          if (id.includes('brotli')) return 'brotli'
        },
      },
    },
  },
  
  // Prevent vite from obscuring Rust errors
  clearScreen: false,
  
  // Tauri expects a fixed port, fail if that port is not available
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      // Tell vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
})
