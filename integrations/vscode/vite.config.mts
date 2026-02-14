import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { resolve } from 'path';

export default defineConfig({
    // Relative asset URLs are required for VSCode/Cursor webviews.
    // Absolute `/assets/...` paths can fail with ERR_ACCESS_DENIED behind webview service workers.
    base: './',
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
            output: {
                manualChunks(id) {
                    if (id.includes('node_modules/reactflow')) {
                        return 'vendor-reactflow';
                    }
                    if (
                        id.includes('react-markdown') ||
                        id.includes('rehype-highlight') ||
                        id.includes('highlight.js')
                    ) {
                        return 'vendor-markdown';
                    }
                    if (id.includes('node_modules/lucide-react')) {
                        return 'vendor-icons';
                    }
                    if (id.includes('node_modules')) {
                        return 'vendor';
                    }
                    return undefined;
                },
            },
        },
        emptyOutDir: true,
    },
    root: resolve(__dirname, 'webview-ui'),
});
