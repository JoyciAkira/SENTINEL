import { marked } from 'marked';
import hljs from 'highlight.js/lib/core';
import typescript from 'highlight.js/lib/languages/typescript';
import rust from 'highlight.js/lib/languages/rust';
import python from 'highlight.js/lib/languages/python';
import json from 'highlight.js/lib/languages/json';
import bash from 'highlight.js/lib/languages/bash';

// Register only needed languages for tree-shaking
hljs.registerLanguage('typescript', typescript);
hljs.registerLanguage('javascript', typescript); // TS superset
hljs.registerLanguage('rust', rust);
hljs.registerLanguage('python', python);
hljs.registerLanguage('json', json);
hljs.registerLanguage('bash', bash);

marked.setOptions({
    gfm: true,
    breaks: true,
});

function escapeHtml(value: string): string {
    return value
        .replace(/&/g, '&amp;')
        .replace(/</g, '&lt;')
        .replace(/>/g, '&gt;')
        .replace(/"/g, '&quot;')
        .replace(/'/g, '&#39;');
}

const renderer = new marked.Renderer();
renderer.code = function (...args: any[]) {
    let text = '';
    let lang: string | undefined;

    // Marked can call renderer.code with different signatures depending on version.
    // Support both object form ({ text, lang }) and positional form (code, infostring).
    const first = args[0];
    if (first && typeof first === 'object' && ('text' in first || 'raw' in first)) {
        text = String(first.text ?? first.raw ?? '');
        lang = typeof first.lang === 'string' ? first.lang : undefined;
    } else {
        text = String(first ?? '');
        lang = typeof args[1] === 'string' ? args[1] : undefined;
    }

    const normalizedLang = (lang ?? '').trim().toLowerCase();
    const supportedLanguage =
        normalizedLang && hljs.getLanguage(normalizedLang) ? normalizedLang : null;
    const language = supportedLanguage ?? 'plaintext';
    let highlighted: string;
    if (supportedLanguage) {
        try {
            highlighted = hljs.highlight(text, { language: supportedLanguage }).value;
        } catch {
            highlighted = escapeHtml(text);
        }
    } else {
        highlighted = escapeHtml(text);
    }
    return `<pre><code class="hljs language-${language}">${highlighted}</code></pre>`;
};

renderer.html = function (...args: any[]) {
    const first = args[0];
    const raw =
        typeof first === 'string'
            ? first
            : String(first?.text ?? first?.raw ?? '');
    // Never render raw HTML directly inside the chat bubble.
    // This prevents JSX snippets from being interpreted and disappearing.
    return `<pre><code class="hljs language-html">${escapeHtml(raw)}</code></pre>`;
};

marked.use({ renderer });

export function renderMarkdown(text: string): string {
    return marked.parse(text) as string;
}
