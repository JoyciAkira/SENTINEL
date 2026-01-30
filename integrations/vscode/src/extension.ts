import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
import { MCPClient } from './mcp/client';
import { AlignmentProvider } from './providers/alignmentProvider';
import { GoalTreeProvider } from './providers/goalTreeProvider';
import { AgentProvider } from './providers/agentProvider';
import { SecurityProvider } from './providers/securityProvider';
import { NetworkProvider } from './providers/networkProvider';
import { SentinelStatusBar } from './providers/statusBar';
import { SentinelCodeLensProvider } from './providers/codeLensProvider';
import { SentinelChatViewProvider } from './webview-providers/SentinelChatViewProvider';
import {
    CONFIG_SENTINEL_PATH,
    CONFIG_DEFAULT_PATH,
    CMD_OPEN_CHAT,
    CMD_REFRESH_GOALS,
    CMD_VALIDATE_ACTION,
    CMD_SHOW_ALIGNMENT,
    VIEW_ALIGNMENT,
    VIEW_GOALS,
    VIEW_AGENTS,
    VIEW_SECURITY,
    VIEW_NETWORK,
    POLL_INTERVAL_MS,
} from './shared/constants';

let lspClient: LanguageClient | undefined;
let mcpClient: MCPClient | undefined;
let pollTimer: ReturnType<typeof setInterval> | undefined;

export function activate(context: vscode.ExtensionContext) {
    const outputChannel = vscode.window.createOutputChannel('Sentinel');
    context.subscriptions.push(outputChannel);

    let sentinelPath = vscode.workspace.getConfiguration('sentinel').get<string>(CONFIG_SENTINEL_PATH) || CONFIG_DEFAULT_PATH;
    sentinelPath = sentinelPath.replace(/^["'](.+)["']$/, '$1');

    const workspaceRoot = vscode.workspace.workspaceFolders
        ? vscode.workspace.workspaceFolders[0].uri.fsPath
        : '.';

    // ── 1. MCP Client ────────────────────────────────────────
    mcpClient = new MCPClient(sentinelPath, workspaceRoot, outputChannel);

    // Start MCP connection (non-blocking, will reconnect on failure)
    mcpClient.start().catch((err) => {
        outputChannel.appendLine(`MCP initial connection failed: ${err.message}`);
    });

    // ── 2. LSP Client ────────────────────────────────────────
    const serverOptions: ServerOptions = {
        command: sentinelPath,
        args: ['lsp'],
        options: { cwd: workspaceRoot },
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'python' },
        ],
    };

    lspClient = new LanguageClient('sentinelLSP', 'Sentinel LSP', serverOptions, clientOptions);
    lspClient.start();

    // ── 3. TreeView Providers ────────────────────────────────
    const alignmentProvider = new AlignmentProvider(mcpClient);
    const goalTreeProvider = new GoalTreeProvider(mcpClient, sentinelPath, workspaceRoot);
    const agentProvider = new AgentProvider(mcpClient, sentinelPath, workspaceRoot);
    const securityProvider = new SecurityProvider(mcpClient, sentinelPath, workspaceRoot);
    const networkProvider = new NetworkProvider(mcpClient, sentinelPath, workspaceRoot);

    context.subscriptions.push(
        vscode.window.registerTreeDataProvider(VIEW_ALIGNMENT, alignmentProvider),
        vscode.window.registerTreeDataProvider(VIEW_GOALS, goalTreeProvider),
        vscode.window.registerTreeDataProvider(VIEW_AGENTS, agentProvider),
        vscode.window.registerTreeDataProvider(VIEW_SECURITY, securityProvider),
        vscode.window.registerTreeDataProvider(VIEW_NETWORK, networkProvider),
    );

    // ── 4. Status Bar ────────────────────────────────────────
    const statusBar = new SentinelStatusBar(mcpClient);
    context.subscriptions.push({ dispose: () => statusBar.dispose() });

    // ── 5. CodeLens ──────────────────────────────────────────
    const codeLensProvider = new SentinelCodeLensProvider(mcpClient);
    context.subscriptions.push(
        vscode.languages.registerCodeLensProvider(
            [
                { language: 'rust' },
                { language: 'typescript' },
                { language: 'javascript' },
                { language: 'python' },
            ],
            codeLensProvider
        )
    );

    // ── 6. Chat Webview ──────────────────────────────────────
    const chatProvider = new SentinelChatViewProvider(
        context.extensionUri,
        mcpClient,
        outputChannel
    );
    context.subscriptions.push(
        vscode.window.registerWebviewViewProvider(
            SentinelChatViewProvider.viewId,
            chatProvider,
            { webviewOptions: { retainContextWhenHidden: true } }
        )
    );

    // ── 7. Commands ──────────────────────────────────────────
    context.subscriptions.push(
        vscode.commands.registerCommand(CMD_OPEN_CHAT, () => {
            vscode.commands.executeCommand('sentinel-chat.focus');
        }),

        vscode.commands.registerCommand(CMD_REFRESH_GOALS, () => {
            alignmentProvider.refresh();
            goalTreeProvider.refresh();
            agentProvider.refresh();
            securityProvider.refresh();
            networkProvider.refresh();
        }),

        vscode.commands.registerCommand(CMD_VALIDATE_ACTION, async () => {
            if (!mcpClient?.connected) {
                vscode.window.showWarningMessage('Sentinel is not connected.');
                return;
            }

            const description = await vscode.window.showInputBox({
                prompt: 'Describe the action to validate',
                placeHolder: 'e.g., Implement JWT authentication',
            });

            if (!description) return;

            try {
                const result = await mcpClient.validateAction('user_action', description);
                const msg = `Alignment: ${result.alignment_score.toFixed(1)}% | Risk: ${result.risk_level} | ${result.approved ? 'APPROVED' : 'REJECTED'}: ${result.rationale}`;
                if (result.approved) {
                    vscode.window.showInformationMessage(msg);
                } else {
                    vscode.window.showWarningMessage(msg);
                }
            } catch (err: any) {
                vscode.window.showErrorMessage(`Validation failed: ${err.message}`);
            }
        }),

        vscode.commands.registerCommand(CMD_SHOW_ALIGNMENT, async () => {
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
            } catch (err: any) {
                vscode.window.showErrorMessage(`Failed to get alignment: ${err.message}`);
            }
        }),
    );

    // ── 8. Polling ───────────────────────────────────────────
    pollTimer = setInterval(async () => {
        if (!mcpClient?.connected) return;

        try {
            const report = await mcpClient.getAlignment();

            // Update all consumers
            statusBar.update(report);
            codeLensProvider.updateReport(report);
            alignmentProvider.updateReport(report);
            chatProvider.updateAlignment(report);
        } catch {
            // Silently fail on poll; will retry next interval
        }
    }, POLL_INTERVAL_MS);

    // ── 9. Connection status updates ─────────────────────────
    mcpClient.on('connected', () => {
        outputChannel.appendLine('Sentinel MCP connected');
        // Trigger initial data load
        vscode.commands.executeCommand(CMD_REFRESH_GOALS);
    });

    mcpClient.on('disconnected', () => {
        statusBar.showDisconnected();
        outputChannel.appendLine('Sentinel MCP disconnected');
    });

    outputChannel.appendLine(`Sentinel extension activated. Path: ${sentinelPath}`);
}

export function deactivate(): Thenable<void> | undefined {
    if (pollTimer) {
        clearInterval(pollTimer);
    }

    mcpClient?.stop();

    if (!lspClient) {
        return undefined;
    }
    return lspClient.stop();
}
