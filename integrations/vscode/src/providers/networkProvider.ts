import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import { spawn } from 'child_process';
import type { StatusReport } from '../shared/types';

class NetworkItem extends vscode.TreeItem {
    constructor(
        label: string,
        description: string,
        collapsible: vscode.TreeItemCollapsibleState = vscode.TreeItemCollapsibleState.None
    ) {
        super(label, collapsible);
        this.description = description;
    }
}

export class NetworkProvider implements vscode.TreeDataProvider<NetworkItem> {
    private _onDidChange = new vscode.EventEmitter<NetworkItem | undefined | void>();
    readonly onDidChangeTreeData = this._onDidChange.event;

    constructor(
        private client: MCPClient,
        private sentinelPath: string,
        private cwd: string
    ) {}

    refresh(): void {
        this._onDidChange.fire();
    }

    getTreeItem(element: NetworkItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: NetworkItem): Promise<NetworkItem[]> {
        if (element) return [];

        if (!this.client.connected) {
            const item = new NetworkItem('Sentinel not connected', 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }

        try {
            const report = await this.runStatusJson();
            const manifold = report?.manifold;
            if (!manifold) return [new NetworkItem('No manifold', '')];

            const items: NetworkItem[] = [];

            const peerCount = manifold.peer_count ?? 0;
            const peerItem = new NetworkItem(
                `Connected Peers: ${peerCount}`,
                peerCount > 0 ? 'ONLINE' : 'SEARCHING'
            );
            peerItem.iconPath = new vscode.ThemeIcon('broadcast');
            items.push(peerItem);

            const consensusItem = new NetworkItem(
                manifold.consensus_active ? 'Consensus: VOTING' : 'Global Consensus Stable',
                manifold.consensus_active ? 'ACTIVE' : 'SYNCED'
            );
            consensusItem.iconPath = new vscode.ThemeIcon(
                manifold.consensus_active ? 'check-all' : 'shield'
            );
            items.push(consensusItem);

            return items;
        } catch {
            return [new NetworkItem('Network Offline', 'OFFLINE')];
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
