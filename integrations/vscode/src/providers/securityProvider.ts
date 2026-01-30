import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import { spawn } from 'child_process';
import type { StatusReport } from '../shared/types';

class SecurityItem extends vscode.TreeItem {
    constructor(
        label: string,
        description: string,
        collapsible: vscode.TreeItemCollapsibleState = vscode.TreeItemCollapsibleState.None
    ) {
        super(label, collapsible);
        this.description = description;
    }
}

export class SecurityProvider implements vscode.TreeDataProvider<SecurityItem> {
    private _onDidChange = new vscode.EventEmitter<SecurityItem | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChange.event;

    constructor(
        private client: MCPClient,
        private sentinelPath: string,
        private cwd: string
    ) {}

    refresh(): void {
        this._onDidChange.fire();
    }

    getTreeItem(element: SecurityItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: SecurityItem): Promise<SecurityItem[]> {
        if (element) return [];

        if (!this.client.connected) {
            const item = new SecurityItem('Sentinel not connected', 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }

        try {
            const report = await this.runStatusJson();
            const external = report?.external;

            const items: SecurityItem[] = [];

            const riskLevel = external ? Math.round(external.risk_level * 100) : 0;
            const riskItem = new SecurityItem(`Risk Level: ${riskLevel}%`, riskLevel === 0 ? 'SECURE' : 'AT RISK');
            riskItem.iconPath = new vscode.ThemeIcon(
                riskLevel === 0 ? 'shield' : 'warning',
                riskLevel === 0
                    ? new vscode.ThemeColor('testing.iconPassed')
                    : new vscode.ThemeColor('notificationsWarningIcon.foreground')
            );
            items.push(riskItem);

            if (external?.alerts && external.alerts.length > 0) {
                for (const alert of external.alerts) {
                    const alertItem = new SecurityItem(alert, 'ALERT');
                    alertItem.iconPath = new vscode.ThemeIcon('warning', new vscode.ThemeColor('notificationsWarningIcon.foreground'));
                    items.push(alertItem);
                }
            } else {
                const safeItem = new SecurityItem('No threats detected', 'SECURE');
                safeItem.iconPath = new vscode.ThemeIcon('pass-filled', new vscode.ThemeColor('testing.iconPassed'));
                items.push(safeItem);
            }

            return items;
        } catch {
            return [new SecurityItem('Error loading security', '')];
        }
    }

    private runStatusJson(): Promise<StatusReport> {
        return new Promise((resolve, reject) => {
            const proc = spawn(this.sentinelPath, ['status', '--json'], {
                cwd: this.cwd,
                shell: false,
            });
            let stdout = '';
            proc.stdout.on('data', (d) => (stdout += d.toString()));
            proc.on('error', (err) => reject(err));
            proc.on('close', (code) => {
                if (code === 0) {
                    try { resolve(JSON.parse(stdout)); }
                    catch { reject(new Error('Parse error')); }
                } else {
                    reject(new Error(`Exit ${code}`));
                }
            });
        });
    }
}
