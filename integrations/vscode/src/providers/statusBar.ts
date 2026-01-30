import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import type { AlignmentReport } from '../shared/types';
import {
    ALIGNMENT_GOOD,
    ALIGNMENT_CONCERNING,
    CMD_SHOW_ALIGNMENT,
} from '../shared/constants';

export class SentinelStatusBar {
    private item: vscode.StatusBarItem;

    constructor(private client: MCPClient) {
        this.item = vscode.window.createStatusBarItem(
            vscode.StatusBarAlignment.Right,
            100
        );
        this.item.command = CMD_SHOW_ALIGNMENT;
        this.showDisconnected();
        this.item.show();
    }

    update(report: AlignmentReport): void {
        const score = report.score.toFixed(0);
        this.item.text = `$(shield) ${score}%`;
        this.item.tooltip = `Sentinel Alignment: ${score}% (${report.status})\nConfidence: ${(report.confidence * 100).toFixed(0)}%\nClick for full report`;

        if (report.score >= ALIGNMENT_GOOD) {
            this.item.backgroundColor = undefined;
        } else if (report.score >= ALIGNMENT_CONCERNING) {
            this.item.backgroundColor = new vscode.ThemeColor('statusBarItem.warningBackground');
        } else {
            this.item.backgroundColor = new vscode.ThemeColor('statusBarItem.errorBackground');
        }
    }

    showDisconnected(): void {
        this.item.text = '$(shield) --';
        this.item.tooltip = 'Sentinel: Not connected';
        this.item.backgroundColor = undefined;
    }

    dispose(): void {
        this.item.dispose();
    }
}
