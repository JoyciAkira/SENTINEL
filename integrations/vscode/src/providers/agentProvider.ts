import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import { spawn } from 'child_process';
import type { StatusReport } from '../shared/types';

class AgentItem extends vscode.TreeItem {
    constructor(
        label: string,
        description: string,
        collapsible: vscode.TreeItemCollapsibleState = vscode.TreeItemCollapsibleState.None
    ) {
        super(label, collapsible);
        this.description = description;
    }
}

export class AgentProvider implements vscode.TreeDataProvider<AgentItem> {
    private _onDidChange = new vscode.EventEmitter<AgentItem | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChange.event;

    constructor(
        private client: MCPClient,
        private sentinelPath: string,
        private cwd: string
    ) {}

    refresh(): void {
        this._onDidChange.fire();
    }

    getTreeItem(element: AgentItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: AgentItem): Promise<AgentItem[]> {
        if (element) return [];

        if (!this.client.connected) {
            const item = new AgentItem('Sentinel not connected', 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }

        try {
            const report = await this.runStatusJson();
            const manifold = report?.manifold;
            if (!manifold) return [new AgentItem('No manifold', '')];

            const items: AgentItem[] = [];

            // File locks = active agents
            const locks = manifold.file_locks ?? {};
            const lockEntries = Object.entries(locks);

            if (lockEntries.length > 0) {
                for (const [file, agentId] of lockEntries) {
                    const shortId = String(agentId).substring(0, 8);
                    const fileName = file.split('/').pop() ?? file;
                    const item = new AgentItem(`Agent ${shortId}`, `Working on ${fileName}`);
                    item.iconPath = new vscode.ThemeIcon('account');
                    items.push(item);
                }
            } else {
                const item = new AgentItem('No active agents', 'IDLE');
                item.iconPath = new vscode.ThemeIcon('circle-slash');
                items.push(item);
            }

            // Handover trail
            const handovers = manifold.handover_log ?? [];
            if (handovers.length > 0) {
                const recent = handovers.slice(-3).reverse();
                for (const note of recent) {
                    const item = new AgentItem(note.content, 'NOTE');
                    item.iconPath = new vscode.ThemeIcon('note');
                    items.push(item);
                }
            }

            return items;
        } catch {
            return [new AgentItem('Error loading agents', '')];
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
