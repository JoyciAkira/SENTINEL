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
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
const client_1 = require("./mcp/client");
const alignmentProvider_1 = require("./providers/alignmentProvider");
const goalTreeProvider_1 = require("./providers/goalTreeProvider");
const agentProvider_1 = require("./providers/agentProvider");
const securityProvider_1 = require("./providers/securityProvider");
const networkProvider_1 = require("./providers/networkProvider");
const statusBar_1 = require("./providers/statusBar");
const codeLensProvider_1 = require("./providers/codeLensProvider");
const SentinelChatViewProvider_1 = require("./webview-providers/SentinelChatViewProvider");
const constants_1 = require("./shared/constants");
let lspClient;
let mcpClient;
let pollTimer;
function activate(context) {
    const outputChannel = vscode.window.createOutputChannel('Sentinel');
    context.subscriptions.push(outputChannel);
    let sentinelPath = vscode.workspace.getConfiguration('sentinel').get(constants_1.CONFIG_SENTINEL_PATH) || constants_1.CONFIG_DEFAULT_PATH;
    sentinelPath = sentinelPath.replace(/^["'](.+)["']$/, '$1');
    const workspaceRoot = vscode.workspace.workspaceFolders
        ? vscode.workspace.workspaceFolders[0].uri.fsPath
        : '.';
    // ── 1. MCP Client ────────────────────────────────────────
    mcpClient = new client_1.MCPClient(sentinelPath, workspaceRoot, outputChannel);
    // Start MCP connection (non-blocking, will reconnect on failure)
    mcpClient.start().catch((err) => {
        outputChannel.appendLine(`MCP initial connection failed: ${err.message}`);
    });
    // ── 2. LSP Client ────────────────────────────────────────
    const serverOptions = {
        command: sentinelPath,
        args: ['lsp'],
        options: { cwd: workspaceRoot },
    };
    const clientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'python' },
        ],
    };
    lspClient = new node_1.LanguageClient('sentinelLSP', 'Sentinel LSP', serverOptions, clientOptions);
    lspClient.start();
    // ── 3. TreeView Providers ────────────────────────────────
    const alignmentProvider = new alignmentProvider_1.AlignmentProvider(mcpClient);
    const goalTreeProvider = new goalTreeProvider_1.GoalTreeProvider(mcpClient, sentinelPath, workspaceRoot);
    const agentProvider = new agentProvider_1.AgentProvider(mcpClient, sentinelPath, workspaceRoot);
    const securityProvider = new securityProvider_1.SecurityProvider(mcpClient, sentinelPath, workspaceRoot);
    const networkProvider = new networkProvider_1.NetworkProvider(mcpClient, sentinelPath, workspaceRoot);
    context.subscriptions.push(vscode.window.registerTreeDataProvider(constants_1.VIEW_ALIGNMENT, alignmentProvider), vscode.window.registerTreeDataProvider(constants_1.VIEW_GOALS, goalTreeProvider), vscode.window.registerTreeDataProvider(constants_1.VIEW_AGENTS, agentProvider), vscode.window.registerTreeDataProvider(constants_1.VIEW_SECURITY, securityProvider), vscode.window.registerTreeDataProvider(constants_1.VIEW_NETWORK, networkProvider));
    // ── 4. Status Bar ────────────────────────────────────────
    const statusBar = new statusBar_1.SentinelStatusBar(mcpClient);
    context.subscriptions.push({ dispose: () => statusBar.dispose() });
    // ── 5. CodeLens ──────────────────────────────────────────
    const codeLensProvider = new codeLensProvider_1.SentinelCodeLensProvider(mcpClient);
    context.subscriptions.push(vscode.languages.registerCodeLensProvider([
        { language: 'rust' },
        { language: 'typescript' },
        { language: 'javascript' },
        { language: 'python' },
    ], codeLensProvider));
    // ── 6. Chat Webview ──────────────────────────────────────
    const chatProvider = new SentinelChatViewProvider_1.SentinelChatViewProvider(context.extensionUri, mcpClient, outputChannel);
    context.subscriptions.push(vscode.window.registerWebviewViewProvider(SentinelChatViewProvider_1.SentinelChatViewProvider.viewId, chatProvider, { webviewOptions: { retainContextWhenHidden: true } }));
    // ── 7. Commands ──────────────────────────────────────────
    context.subscriptions.push(vscode.commands.registerCommand(constants_1.CMD_OPEN_CHAT, () => {
        vscode.commands.executeCommand('sentinel-chat.focus');
    }), vscode.commands.registerCommand(constants_1.CMD_REFRESH_GOALS, () => {
        alignmentProvider.refresh();
        goalTreeProvider.refresh();
        agentProvider.refresh();
        securityProvider.refresh();
        networkProvider.refresh();
    }), vscode.commands.registerCommand(constants_1.CMD_VALIDATE_ACTION, async () => {
        if (!mcpClient?.connected) {
            vscode.window.showWarningMessage('Sentinel is not connected.');
            return;
        }
        const description = await vscode.window.showInputBox({
            prompt: 'Describe the action to validate',
            placeHolder: 'e.g., Implement JWT authentication',
        });
        if (!description)
            return;
        try {
            const result = await mcpClient.validateAction('user_action', description);
            const msg = `Alignment: ${result.alignment_score.toFixed(1)}% | Risk: ${result.risk_level} | ${result.approved ? 'APPROVED' : 'REJECTED'}: ${result.rationale}`;
            if (result.approved) {
                vscode.window.showInformationMessage(msg);
            }
            else {
                vscode.window.showWarningMessage(msg);
            }
        }
        catch (err) {
            vscode.window.showErrorMessage(`Validation failed: ${err.message}`);
        }
    }), vscode.commands.registerCommand(constants_1.CMD_SHOW_ALIGNMENT, async () => {
        if (!mcpClient?.connected) {
            vscode.window.showWarningMessage('Sentinel is not connected. Ensure sentinel-cli is installed and accessible.');
            return;
        }
        try {
            const report = await mcpClient.getAlignment();
            const lines = [
                `Score: ${report.score.toFixed(1)}%`,
                `Confidence: ${(report.confidence * 100).toFixed(0)}%`,
                `Status: ${report.status}`,
            ];
            if (report.violations.length > 0) {
                lines.push('', 'Violations:');
                for (const v of report.violations) {
                    lines.push(`  - ${v.description} (severity: ${v.severity})`);
                }
            }
            vscode.window.showInformationMessage(lines.join('\n'), { modal: true });
        }
        catch (err) {
            vscode.window.showErrorMessage(`Failed to get alignment: ${err.message}`);
        }
    }));
    // ── 8. Polling ───────────────────────────────────────────
    pollTimer = setInterval(async () => {
        if (!mcpClient?.connected)
            return;
        try {
            const report = await mcpClient.getAlignment();
            // Update all consumers
            statusBar.update(report);
            codeLensProvider.updateReport(report);
            alignmentProvider.updateReport(report);
            chatProvider.updateAlignment(report);
        }
        catch {
            // Silently fail on poll; will retry next interval
        }
    }, constants_1.POLL_INTERVAL_MS);
    // ── 9. Connection status updates ─────────────────────────
    mcpClient.on('connected', () => {
        outputChannel.appendLine('Sentinel MCP connected');
        // Trigger initial data load
        vscode.commands.executeCommand(constants_1.CMD_REFRESH_GOALS);
    });
    mcpClient.on('disconnected', () => {
        statusBar.showDisconnected();
        outputChannel.appendLine('Sentinel MCP disconnected');
    });
    outputChannel.appendLine(`Sentinel extension activated. Path: ${sentinelPath}`);
}
function deactivate() {
    if (pollTimer) {
        clearInterval(pollTimer);
    }
    mcpClient?.stop();
    if (!lspClient) {
        return undefined;
    }
    return lspClient.stop();
}
//# sourceMappingURL=extension.js.map