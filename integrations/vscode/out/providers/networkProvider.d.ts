import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
declare class NetworkItem extends vscode.TreeItem {
    constructor(label: string, description: string, collapsible?: vscode.TreeItemCollapsibleState);
}
export declare class NetworkProvider implements vscode.TreeDataProvider<NetworkItem> {
    private client;
    private sentinelPath;
    private cwd;
    private _onDidChange;
    readonly onDidChangeTreeData: vscode.Event<void | NetworkItem | undefined>;
    constructor(client: MCPClient, sentinelPath: string, cwd: string);
    refresh(): void;
    getTreeItem(element: NetworkItem): vscode.TreeItem;
    getChildren(element?: NetworkItem): Promise<NetworkItem[]>;
    private runStatusJson;
}
export {};
