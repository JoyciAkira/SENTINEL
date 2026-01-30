import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
declare class SecurityItem extends vscode.TreeItem {
    constructor(label: string, description: string, collapsible?: vscode.TreeItemCollapsibleState);
}
export declare class SecurityProvider implements vscode.TreeDataProvider<SecurityItem> {
    private client;
    private sentinelPath;
    private cwd;
    private _onDidChange;
    readonly onDidChangeTreeData: vscode.Event<void | SecurityItem | undefined>;
    constructor(client: MCPClient, sentinelPath: string, cwd: string);
    refresh(): void;
    getTreeItem(element: SecurityItem): vscode.TreeItem;
    getChildren(element?: SecurityItem): Promise<SecurityItem[]>;
    private runStatusJson;
}
export {};
