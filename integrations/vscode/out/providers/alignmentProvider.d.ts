import * as vscode from 'vscode';
import { MCPClient } from '../mcp/client';
import type { AlignmentReport } from '../shared/types';
declare class AlignmentItem extends vscode.TreeItem {
    children?: AlignmentItem[] | undefined;
    constructor(label: string, description: string, collapsible?: vscode.TreeItemCollapsibleState, children?: AlignmentItem[] | undefined);
}
export declare class AlignmentProvider implements vscode.TreeDataProvider<AlignmentItem> {
    private client;
    private _onDidChange;
    readonly onDidChangeTreeData: vscode.Event<void | AlignmentItem | undefined>;
    private lastReport;
    constructor(client: MCPClient);
    refresh(): void;
    updateReport(report: AlignmentReport): void;
    getTreeItem(element: AlignmentItem): vscode.TreeItem;
    getChildren(element?: AlignmentItem): Promise<AlignmentItem[]>;
}
export {};
