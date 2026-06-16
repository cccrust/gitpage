import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': 'http://localhost:8080',
      '/git': 'http://localhost:8080',
      '/pages': 'http://localhost:8080',
    },
  },
})
