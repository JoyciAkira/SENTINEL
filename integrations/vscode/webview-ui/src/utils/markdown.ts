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

const renderer = new marked.Renderer();
renderer.code = function ({ text, lang }: { text: string; lang?: string }) {
    const language = lang && hljs.getLanguage(lang) ? lang : 'plaintext';
    let highlighted: string;
    try {
        highlighted = hljs.highlight(text, { language }).value;
    } catch {
        highlighted = text;
    }
    return `<pre><code class="hljs language-${language}">${highlighted}</code></pre>`;
};

marked.use({ renderer });

export function renderMarkdown(text: string): string {
    return marked.parse(text) as string;
}
