"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.getWebviewContent = getWebviewContent;
const vscode = __importStar(require("vscode"));
const path = __importStar(require("path"));
const fs = __importStar(require("fs"));
/**
 * Returns the HTML content for the Sentinel Chat webview.
 * In development, loads from Vite dev server.
 * In production, loads the built assets from out/webview/.
 */
function getWebviewContent(webview, extensionUri) {
    const webviewDir = vscode.Uri.joinPath(extensionUri, 'out', 'webview');
    // Check if production build exists
    const indexPath = path.join(webviewDir.fsPath, 'index.html');
    if (fs.existsSync(indexPath)) {
        return getProductionHtml(webview, extensionUri, indexPath);
    }
    // Fallback: development placeholder
    return getDevelopmentHtml();
}
function getProductionHtml(webview, extensionUri, indexPath) {
    let html = fs.readFileSync(indexPath, 'utf-8');
    const webviewUri = webview.asWebviewUri(vscode.Uri.joinPath(extensionUri, 'out', 'webview'));
    // Rewrite asset paths to use webview URIs
    html = html.replace(/(href|src)="(\/assets\/[^"]+)"/g, (_match, attr, assetPath) => {
        return `${attr}="${webviewUri}${assetPath}"`;
    });
    // Add CSP meta tag
    const nonce = getNonce();
    const csp = [
        `default-src 'none'`,
        `img-src ${webview.cspSource} data:`,
        `script-src 'nonce-${nonce}'`,
        `style-src ${webview.cspSource} 'unsafe-inline'`,
        `font-src ${webview.cspSource}`,
    ].join('; ');
    html = html.replace('<head>', `<head>\n<meta http-equiv="Content-Security-Policy" content="${csp}">`);
    // Add nonce to script tags
    html = html.replace(/<script /g, `<script nonce="${nonce}" `);
    return html;
}
function getDevelopmentHtml() {
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
function getNonce() {
    let text = '';
    const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
        text += chars.charAt(Math.floor(Math.random() * chars.length));
    }
    return text;
}
//# sourceMappingURL=getWebviewContent.js.map