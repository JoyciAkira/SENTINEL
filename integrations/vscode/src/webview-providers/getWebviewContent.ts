import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

/**
 * Returns the HTML content for the Sentinel Chat webview.
 * In development, loads from Vite dev server.
 * In production, loads the built assets from out/webview/.
 */
export function getWebviewContent(
    webview: vscode.Webview,
    extensionUri: vscode.Uri
): string {
    const webviewDir = vscode.Uri.joinPath(extensionUri, 'out', 'webview');

    // Check if production build exists
    const indexPath = path.join(webviewDir.fsPath, 'index.html');
    if (fs.existsSync(indexPath)) {
        return getProductionHtml(webview, extensionUri, indexPath);
    }

    // Fallback: development placeholder
    return getDevelopmentHtml();
}

function getProductionHtml(
    webview: vscode.Webview,
    extensionUri: vscode.Uri,
    indexPath: string
): string {
    let html = fs.readFileSync(indexPath, 'utf-8');

    // Rewrite asset paths to use webview URIs.
    // Supports /assets/, assets/, and ./assets/ to stay robust across Vite base modes.
    html = html.replace(
        /(href|src)="((?:\.\/)?\/?(?:assets\/)[^"]+)"/g,
        (_match, attr, assetPath) => {
            const normalized = assetPath.replace(/^\.\//, '');
            const relativeAssetPath = normalized.replace(/^\/+/, '');
            const assetUri = webview.asWebviewUri(
                vscode.Uri.joinPath(extensionUri, 'out', 'webview', ...relativeAssetPath.split('/'))
            );
            return `${attr}="${assetUri.toString()}"`;
        }
    );

    const nonce = getNonce();

    // Trusted Types: default policy so bundled code (e.g. new Function) is allowed in strict environments (Cursor)
    const trustedTypesScript = [
        `<script nonce="${nonce}">`,
        "(function(){",
        "if (typeof window.trustedTypes !== 'undefined' && window.trustedTypes.createPolicy) {",
        "  try {",
        "    window.trustedTypes.createPolicy('default', {",
        "      createScript: function(s) { return s; },",
        "      createScriptURL: function(s) { return s; }",
        "    });",
        "  } catch(e) {}",
        "}",
        "})();",
        "</script>",
    ].join('');

    // CSP: allow nonced bootstrap + webview-origin module chunks (Vite dynamic imports/preloads).
    // Keep TT policy bootstrap allowed without forcing require-trusted-types-for.
    const csp = [
        `default-src 'none'`,
        `base-uri 'none'`,
        `object-src 'none'`,
        `img-src ${webview.cspSource} data:`,
        `script-src ${webview.cspSource} 'nonce-${nonce}'`,
        `connect-src ${webview.cspSource} https:`,
        `worker-src ${webview.cspSource} blob:`,
        `style-src ${webview.cspSource} 'unsafe-inline'`,
        `font-src ${webview.cspSource}`,
    ].join('; ');

    html = html.replace(
        '<head>',
        `<head>\n<meta http-equiv="Content-Security-Policy" content="${csp}">\n<meta property="csp-nonce" nonce="${nonce}">\n${trustedTypesScript}`
    );

    // Add nonce to script tags that don't already have it (e.g. module bundle)
    html = html.replace(/<script (?![^>]*\bnonce=)/g, `<script nonce="${nonce}" `);

    return html;
}

function getDevelopmentHtml(): string {
    return `<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <style>
        body {
            font-family: var(--vscode-font-family);
            color: var(--vscode-foreground);
            background: var(--vscode-sideBar-background);
            display: flex;
            align-items: center;
            justify-content: center;
            height: 100vh;
            margin: 0;
            padding: 20px;
            text-align: center;
        }
    </style>
</head>
<body>
    <div>
        <h3>Sentinel Chat</h3>
        <p>Webview not built. Run:</p>
        <code>cd integrations/vscode && npm run build:webview</code>
    </div>
</body>
</html>`;
}

function getNonce(): string {
    let text = '';
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return text;
}
