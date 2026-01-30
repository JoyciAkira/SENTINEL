import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
declare class AgentItem extends vscode.TreeItem {
    constructor(label: string, description: string, collapsible?: vscode.TreeItemCollapsibleState);
}
export declare class AgentProvider implements vscode.TreeDataProvider<AgentItem> {
    private client;
    private sentinelPath;
    private cwd;
    private _onDidChange;
    readonly onDidChangeTreeData: vscode.Event<void | AgentItem | undefined>;
    constructor(client: MCPClient, sentinelPath: string, cwd: string);
    refresh(): void;
    getTreeItem(element: AgentItem): vscode.TreeItem;
    getChildren(element?: AgentItem): Promise<AgentItem[]>;
    private runStatusJson;
}
export {};
