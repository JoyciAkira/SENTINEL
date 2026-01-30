import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import type { AlignmentReport } from '../shared/types';
export declare class SentinelCodeLensProvider implements vscode.CodeLensProvider {
    private client;
    private _onDidChange;
    readonly onDidChangeCodeLenses: vscode.Event<void>;
    private lastReport;
    constructor(client: MCPClient);
    updateReport(report: AlignmentReport): void;
    provideCodeLenses(document: vscode.TextDocument): vscode.CodeLens[];
}
