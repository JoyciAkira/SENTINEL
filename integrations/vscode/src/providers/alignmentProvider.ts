import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import type { AlignmentReport } from '../shared/types';
import {
    ALIGNMENT_EXCELLENT,
    ALIGNMENT_GOOD,
    ALIGNMENT_CONCERNING,
} from '../shared/constants';

class AlignmentItem extends vscode.TreeItem {
    constructor(
        label: string,
        description: string,
        collapsible: vscode.TreeItemCollapsibleState = vscode.TreeItemCollapsibleState.None,
        public children?: AlignmentItem[]
    ) {
        super(label, collapsible);
        this.description = description;
    }
}

export class AlignmentProvider implements vscode.TreeDataProvider<AlignmentItem> {
    private _onDidChange = new vscode.EventEmitter<AlignmentItem | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChange.event;

    private lastReport: AlignmentReport | null = null;

    constructor(private client: MCPClient) {}

    refresh(): void {
        this._onDidChange.fire();
    }

    updateReport(report: AlignmentReport): void {
        this.lastReport = report;
        this.refresh();
    }

    getTreeItem(element: AlignmentItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: AlignmentItem): Promise<AlignmentItem[]> {
        if (element) return element.children ?? [];

        if (!this.client.connected) {
            const item = new AlignmentItem('Sentinel not connected', 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }

        try {
            if (!this.lastReport) {
                this.lastReport = await this.client.getAlignment();
            }
            const r = this.lastReport;

            const scoreItem = new AlignmentItem(
                `Score: ${r.score.toFixed(1)}%`,
                r.status
            );
            scoreItem.iconPath = new vscode.ThemeIcon(
                r.score >= ALIGNMENT_EXCELLENT ? 'pass-filled' :
                r.score >= ALIGNMENT_GOOD ? 'pass' :
                r.score >= ALIGNMENT_CONCERNING ? 'warning' : 'error',
                r.score >= ALIGNMENT_GOOD
                    ? new vscode.ThemeColor('testing.iconPassed')
                    : r.score >= ALIGNMENT_CONCERNING
                    ? new vscode.ThemeColor('notificationsWarningIcon.foreground')
                    : new vscode.ThemeColor('testing.iconFailed')
            );

            const confItem = new AlignmentItem(
                `Confidence: ${(r.confidence * 100).toFixed(0)}%`,
                ''
            );
            confItem.iconPath = new vscode.ThemeIcon('graph');

            const items: AlignmentItem[] = [scoreItem, confItem];

            if (r.violations.length > 0) {
                const violationChildren = r.violations.map(v => {
                    const vi = new AlignmentItem(v.description, `Severity: ${v.severity}`);
                    vi.iconPath = new vscode.ThemeIcon('warning', new vscode.ThemeColor('notificationsWarningIcon.foreground'));
                    return vi;
                });
                const violRoot = new AlignmentItem(
                    `Violations (${r.violations.length})`,
                    '',
                    vscode.TreeItemCollapsibleState.Expanded,
                    violationChildren
                );
                violRoot.iconPath = new vscode.ThemeIcon('issues');
                items.push(violRoot);
            }

            return items;
        } catch {
            return [new AlignmentItem('Failed to load alignment', 'Retry')];
        }
    }
}
