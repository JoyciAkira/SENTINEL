import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'path';

export default defineConfig({
    plugins: [
        react(),
        tailwindcss(),
    ],
    resolve: {
        alias: {
            '@': resolve(__dirname, 'webview-ui/src'),
        },
    },
    build: {
        outDir: resolve(__dirname, 'out/webview'),
        rollupOptions: {
            input: resolve(__dirname, 'webview-ui/index.html'),
        },
        emptyOutDir: true,
    },
    root: resolve(__dirname, 'webview-ui'),
});
