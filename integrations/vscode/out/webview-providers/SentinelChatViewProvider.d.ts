import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import type { AlignmentReport } from '../shared/types';
/**
 * WebviewViewProvider for the Sentinel Chat sidebar panel.
 * Implements the Cline-style full sidebar chat experience.
 */
export declare class SentinelChatViewProvider implements vscode.WebviewViewProvider {
    private extensionUri;
    private client;
    private outputChannel;
    static readonly viewId = "sentinel-chat";
    private view?;
    constructor(extensionUri: vscode.Uri, client: MCPClient, outputChannel: vscode.OutputChannel);
    resolveWebviewView(webviewView: vscode.WebviewView, _context: vscode.WebviewViewResolveContext, _token: vscode.CancellationToken): void;
    postMessage(msg: unknown): void;
    updateAlignment(report: AlignmentReport): void;
    updateGoals(goals: Array<{
        id: string;
        description: string;
        status: string;
    }>): void;
    private handleWebviewMessage;
    private handleChatMessage;
}
