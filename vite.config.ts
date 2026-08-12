import { defineConfig } from 'vite'
import { svelte } from '@sveltejs/vite-plugin-svelte'
import tailwindcss from '@tailwindcss/vite'

// https://vitejs.dev/config/
export default defineConfig({
    plugins: [
        svelte(),
        tailwindcss(),
    ],
    resolve: {
        alias: {
            $lib: '/src/lib',
        }
    },
    server: {
        host: '0.0.0.0',
        port: 1420,
        strictPort: true,
    }
})
