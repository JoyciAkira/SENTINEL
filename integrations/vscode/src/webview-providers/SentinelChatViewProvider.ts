import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import { getWebviewContent } from './getWebviewContent';
import type { AlignmentReport } from '../shared/types';

/**
 * WebviewViewProvider for the Sentinel Chat sidebar panel.
 * Implements the Cline-style full sidebar chat experience.
 */
export class SentinelChatViewProvider implements vscode.WebviewViewProvider {
    public static readonly viewId = 'sentinel-chat';

    private view?: vscode.WebviewView;

    constructor(
        private extensionUri: vscode.Uri,
        private client: MCPClient,
        private outputChannel: vscode.OutputChannel
    ) {}

    resolveWebviewView(
        webviewView: vscode.WebviewView,
        _context: vscode.WebviewViewResolveContext,
        _token: vscode.CancellationToken
    ): void {
        this.view = webviewView;

        webviewView.webview.options = {
            enableScripts: true,
            localResourceRoots: [
                vscode.Uri.joinPath(this.extensionUri, 'out', 'webview'),
            ],
        };

        webviewView.webview.html = getWebviewContent(
            webviewView.webview,
            this.extensionUri
        );

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

    postMessage(msg: unknown): void {
        this.view?.webview.postMessage(msg);
    }

    updateAlignment(report: AlignmentReport): void {
        this.postMessage({
            type: 'alignmentUpdate',
            score: report.score,
            confidence: report.confidence,
            status: report.status,
        });
    }

    updateGoals(goals: Array<{ id: string; description: string; status: string }>): void {
        this.postMessage({
            type: 'goalsUpdate',
            goals,
        });
    }

    private async handleWebviewMessage(msg: any): Promise<void> {
        switch (msg.type) {
            case 'chatMessage':
                await this.handleChatMessage(msg.text);
                break;

            case 'fileApproval':
                this.outputChannel.appendLine(
                    `File ${msg.approved ? 'approved' : 'rejected'}: ${msg.path}`
                );
                break;
        }
    }

    private async handleChatMessage(text: string): Promise<void> {
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
            const parts: string[] = [];

            // First, check alignment
            try {
                const alignment = await this.client.getAlignment();
                parts.push(`**Current Alignment:** ${alignment.score.toFixed(1)}% (${alignment.status})`);
            } catch {
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
                    messageId: '',  // will be set by the response
                    name: 'validate_action',
                    arguments: { action_type: 'user_request', description: text },
                    result: JSON.stringify(validation, null, 2),
                    status: 'success',
                });
            } catch {
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
            } catch {
                // Strategy is optional
            }

            const responseContent = parts.join('\n');
            this.postMessage({
                type: 'chatResponse',
                id: crypto.randomUUID(),
                content: responseContent || 'No response from Sentinel tools.',
            });
        } catch (err: any) {
            this.postMessage({
                type: 'chatResponse',
                id: crypto.randomUUID(),
                content: `Error: ${err.message}`,
            });
        }
    }
}
