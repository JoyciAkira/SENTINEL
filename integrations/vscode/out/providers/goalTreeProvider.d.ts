import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
declare class GoalItem extends vscode.TreeItem {
    children?: GoalItem[];
    constructor(label: string, collapsible: vscode.TreeItemCollapsibleState, description?: string);
}
export declare class GoalTreeProvider implements vscode.TreeDataProvider<GoalItem> {
    private client;
    private sentinelPath;
    private cwd;
    private _onDidChange;
    readonly onDidChangeTreeData: vscode.Event<void | GoalItem | undefined>;
    constructor(client: MCPClient, sentinelPath: string, cwd: string);
    refresh(): void;
    getTreeItem(element: GoalItem): vscode.TreeItem;
    getChildren(element?: GoalItem): Promise<GoalItem[]>;
    private runStatusJson;
}
export {};
