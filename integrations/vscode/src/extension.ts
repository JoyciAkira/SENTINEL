import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    const sentinelPath = vscode.workspace.getConfiguration('sentinel').get<string>('path') || 'sentinel';

    // 1. Setup LSP Client
    const serverOptions: ServerOptions = {
        command: sentinelPath,
        args: ['lsp'],
    };

    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'python' }
        ]
    };

    client = new LanguageClient('sentinelLSP', 'Sentinel LSP', serverOptions, clientOptions);
    client.start();

    // 2. Setup Goal TreeView
    const goalProvider = new SentinelGoalProvider();
    vscode.window.registerTreeDataProvider('sentinel-goals', goalProvider);

    vscode.commands.registerCommand('sentinel.refreshGoals', () => goalProvider.refresh());

    console.log('Sentinel extension and TreeView activated.');
}

class SentinelGoalProvider implements vscode.TreeDataProvider<GoalItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<GoalItem | undefined | null | void> = new vscode.EventEmitter<GoalItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<GoalItem | undefined | null | void> = this._onDidChangeTreeData.event;

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: GoalItem): vscode.TreeItem {
        return element;
    }

    getChildren(element?: GoalItem): Thenable<GoalItem[]> {
        if (element) {
            return Promise.resolve([]);
        } else {
            // Mock data - In futuro richiederà i dati dal backend via LSP o CLI
            return Promise.resolve([
                new GoalItem("Layer 6: Integration", vscode.TreeItemCollapsibleState.None, "90%"),
                new GoalItem("Layer 7: External Awareness", vscode.TreeItemCollapsibleState.None, "0%"),
                new GoalItem("Layer 8: Multi-Agent Sync", vscode.TreeItemCollapsibleState.None, "0%")
            ]);
        }
    }
}

class GoalItem extends vscode.TreeItem {
    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly progress: string
    ) {
        super(label, collapsibleState);
        this.tooltip = `${this.label} - ${this.progress}`;
        this.description = this.progress;
        this.iconPath = new vscode.ThemeIcon('target');
    }
}

export function deactivate(): Thenable<void> | undefined {
    return client ? client.stop() : undefined;
}
