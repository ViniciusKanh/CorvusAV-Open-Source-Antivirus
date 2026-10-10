import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Evita que o Vite limpe a tela, preservando os logs do `cargo`/Tauri.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      // Ignora o diretório do backend Rust para não disparar reload do Vite.
      ignored: ['**/src-tauri/**'],
    },
  },
}))
