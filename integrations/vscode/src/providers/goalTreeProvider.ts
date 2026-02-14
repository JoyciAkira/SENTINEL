import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import type { GoalManifold, GoalNode, GoalStatus, StatusReport } from '../shared/types';
import { spawn } from 'child_process';

class GoalItem extends vscode.TreeItem {
    public children?: GoalItem[];

    constructor(
        label: string,
        collapsible: vscode.TreeItemCollapsibleState,
        description: string = ''
    ) {
        super(label, collapsible);
        this.description = description;
        this.tooltip = `${label} - ${description}`;
    }
}

export class GoalTreeProvider implements vscode.TreeDataProvider<GoalItem> {
    private _onDidChange = new vscode.EventEmitter<GoalItem | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChange.event;

    constructor(
        private client: MCPClient,
        private sentinelPath: string,
        private cwd: string
    ) {}

    refresh(): void {
        this._onDidChange.fire();
    }

    getTreeItem(element: GoalItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: GoalItem): Promise<GoalItem[]> {
        if (element) return element.children ?? [];

        if (!this.client.connected) {
            const item = new GoalItem('Sentinel not connected', vscode.TreeItemCollapsibleState.None, 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }

        try {
            const report = await this.runStatusJson();
            if (!report?.manifold || (report.manifold as any).error) {
                return [new GoalItem('Manifold not initialized', vscode.TreeItemCollapsibleState.None, "Run 'sentinel init'")];
            }

            const manifold = report.manifold;
            const items: GoalItem[] = [];

            // Root intent
            const rootItem = new GoalItem(
                manifold.root_intent.description,
                vscode.TreeItemCollapsibleState.None,
                'ROOT'
            );
            rootItem.iconPath = new vscode.ThemeIcon('target');
            items.push(rootItem);

            // Goal DAG nodes
            if (manifold.goal_dag?.nodes) {
                const nodes = Object.values(manifold.goal_dag.nodes) as GoalNode[];
                const total = nodes.length;
                const completed = nodes.filter(n => n.status === 'Completed').length;

                for (const node of nodes) {
                    const goalItem = new GoalItem(
                        node.description,
                        vscode.TreeItemCollapsibleState.None,
                        node.status
                    );
                    goalItem.iconPath = new vscode.ThemeIcon(statusIcon(node.status));
                    items.push(goalItem);
                }

                // Summary header
                if (total > 0) {
                    const summaryItem = new GoalItem(
                        `Progress: ${completed}/${total}`,
                        vscode.TreeItemCollapsibleState.None,
                        `${Math.round((completed / total) * 100)}%`
                    );
                    summaryItem.iconPath = new vscode.ThemeIcon('pie-chart');
                    items.unshift(summaryItem);
                }
            }

            return items;
        } catch (err: any) {
            return [new GoalItem('Error loading goals', vscode.TreeItemCollapsibleState.None, err.message)];
        }
    }

    private runStatusJson(): Promise<StatusReport> {
        return new Promise((resolve, reject) => {
            const proc = spawn(this.sentinelPath, ['status', '--json'], {
                cwd: this.cwd,
                shell: false,
            });
            let stdout = '';
            let stderr = '';
            proc.stdout.on('data', (d) => (stdout += d.toString()));
            proc.stderr.on('data', (d) => (stderr += d.toString()));
            proc.on('error', (err) => reject(err));
            proc.on('close', (code) => {
                if (code === 0) {
                    try { resolve(JSON.parse(stdout)); }
                    catch (e) { reject(new Error(`JSON parse error: ${stdout}`)); }
                } else {
                    reject(new Error(`Exit code ${code}: ${stderr}`));
                }
            });
        });
    }
}

function statusIcon(status: GoalStatus): string {
    switch (status) {
        case 'Completed': return 'pass-filled';
        case 'InProgress': return 'sync~spin';
        case 'Validating': return 'beaker';
        case 'Ready': return 'circle-outline';
        case 'Pending': return 'circle-outline';
        case 'Blocked': return 'lock';
        case 'Failed': return 'error';
        case 'Deprecated': return 'trash';
        default: return 'question';
    }
}
