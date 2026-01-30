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
exports.SentinelChatViewProvider = void 0;
const vscode = __importStar(require("vscode"));
const getWebviewContent_1 = require("./getWebviewContent");
/**
 * WebviewViewProvider for the Sentinel Chat sidebar panel.
 * Implements the Cline-style full sidebar chat experience.
 */
class SentinelChatViewProvider {
    constructor(extensionUri, client, outputChannel) {
        this.extensionUri = extensionUri;
        this.client = client;
        this.outputChannel = outputChannel;
    }
    resolveWebviewView(webviewView, _context, _token) {
        this.view = webviewView;
        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [
                vscode.Uri.joinPath(this.extensionUri, 'out', 'webview'),
            ],
        };
        webviewView.webview.html = (0, getWebviewContent_1.getWebviewContent)(webviewView.webview, this.extensionUri);
        // Handle messages from webview
        webviewView.webview.onDidReceiveMessage((msg) => {
            this.handleWebviewMessage(msg);
        });
        // Notify webview of connection status
        if (this.client.connected) {
            this.postMessage({ type: 'connected' });
        }
        this.client.on('connected', () => {
            this.postMessage({ type: 'connected' });
        });
        this.client.on('disconnected', () => {
            this.postMessage({ type: 'disconnected' });
        });
    }
    postMessage(msg) {
        this.view?.webview.postMessage(msg);
    }
    updateAlignment(report) {
        this.postMessage({
            type: 'alignmentUpdate',
            score: report.score,
            confidence: report.confidence,
            status: report.status,
        });
    }
    updateGoals(goals) {
        this.postMessage({
            type: 'goalsUpdate',
            goals,
        });
    }
    async handleWebviewMessage(msg) {
        switch (msg.type) {
            case 'chatMessage':
                await this.handleChatMessage(msg.text);
                break;
            case 'fileApproval':
                this.outputChannel.appendLine(`File ${msg.approved ? 'approved' : 'rejected'}: ${msg.path}`);
                break;
        }
    }
    async handleChatMessage(text) {
        if (!this.client.connected) {
            this.postMessage({
                type: 'chatResponse',
                id: crypto.randomUUID(),
                content: 'Sentinel is not connected. Please check that sentinel-cli is installed and accessible.',
            });
            return;
        }
        try {
            // Compose response from available MCP tools
            const parts = [];
            // First, check alignment
            try {
                const alignment = await this.client.getAlignment();
                parts.push(`**Current Alignment:** ${alignment.score.toFixed(1)}% (${alignment.status})`);
            }
            catch {
                parts.push('*Could not retrieve alignment status.*');
            }
            // Validate the user's described action
            try {
                const validation = await this.client.validateAction('user_request', text);
                parts.push('');
                parts.push(`**Action Validation:**`);
                parts.push(`- Alignment Score: ${validation.alignment_score.toFixed(1)}%`);
                parts.push(`- Deviation Probability: ${(validation.deviation_probability * 100).toFixed(0)}%`);
                parts.push(`- Risk Level: ${validation.risk_level}`);
                parts.push(`- ${validation.approved ? 'Approved' : 'Rejected'}: ${validation.rationale}`);
                // Send tool call info
                this.postMessage({
                    type: 'toolCall',
                    messageId: '', // will be set by the response
                    name: 'validate_action',
                    arguments: { action_type: 'user_request', description: text },
                    result: JSON.stringify(validation, null, 2),
                    status: 'success',
                });
            }
            catch {
                parts.push('*Could not validate action.*');
            }
            // Propose strategy if relevant
            try {
                const strategy = await this.client.proposeStrategy(text);
                if (strategy.patterns.length > 0) {
                    parts.push('');
                    parts.push(`**Recommended Strategy** (${(strategy.confidence * 100).toFixed(0)}% confidence):`);
                    for (const pattern of strategy.patterns) {
                        parts.push(`- **${pattern.name}**: ${pattern.description} (${(pattern.success_rate * 100).toFixed(0)}% success)`);
                    }
                }
            }
            catch {
                // Strategy is optional
            }
            const responseContent = parts.join('\n');
            this.postMessage({
                type: 'chatResponse',
                id: crypto.randomUUID(),
                content: responseContent || 'No response from Sentinel tools.',
            });
        }
        catch (err) {
            this.postMessage({
                type: 'chatResponse',
                id: crypto.randomUUID(),
                content: `Error: ${err.message}`,
            });
        }
    }
}
exports.SentinelChatViewProvider = SentinelChatViewProvider;
SentinelChatViewProvider.viewId = 'sentinel-chat';
//# sourceMappingURL=SentinelChatViewProvider.js.map