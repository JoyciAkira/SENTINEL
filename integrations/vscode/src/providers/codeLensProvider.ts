import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import type { AlignmentReport } from '../shared/types';
import { CMD_SHOW_ALIGNMENT } from '../shared/constants';

export class SentinelCodeLensProvider implements vscode.CodeLensProvider {
    private _onDidChange = new vscode.EventEmitter<void>();
    readonly onDidChangeCodeLenses = this._onDidChange.event;

    private lastReport: AlignmentReport | null = null;

    constructor(private client: MCPClient) {}

    updateReport(report: AlignmentReport): void {
        this.lastReport = report;
        this._onDidChange.fire();
    }

    provideCodeLenses(document: vscode.TextDocument): vscode.CodeLens[] {
        const range = new vscode.Range(0, 0, 0, 0);

        if (!this.client.connected) {
            return [
                new vscode.CodeLens(range, {
                    title: '$(shield) Sentinel: Not connected',
                    command: CMD_SHOW_ALIGNMENT,
                }),
            ];
        }

        if (!this.lastReport) {
            return [
                new vscode.CodeLens(range, {
                    title: '$(shield) Sentinel Alignment: ...',
                    command: CMD_SHOW_ALIGNMENT,
                }),
            ];
        }

        const score = this.lastReport.score.toFixed(0);
        return [
            new vscode.CodeLens(range, {
                title: `$(shield) Sentinel Alignment: ${score}%`,
                command: CMD_SHOW_ALIGNMENT,
            }),
        ];
    }
}
