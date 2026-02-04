import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'path';

export default defineConfig({
    plugins: [react()],
    build: {
        outDir: resolve(__dirname, 'out/webview'),
        rollupOptions: {
            input: resolve(__dirname, 'webview-ui/index.html'),
        },
        emptyOutDir: true,
    },
    root: resolve(__dirname, 'webview-ui'),
});
