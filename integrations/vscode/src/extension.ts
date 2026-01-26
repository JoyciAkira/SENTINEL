import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
import { exec } from 'child_process';
import { promisify } from 'util';

const execAsync = promisify(exec);
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
    const goalProvider = new SentinelGoalProvider(sentinelPath, vscode.workspace.rootPath || ".");
    vscode.window.registerTreeDataProvider('sentinel-goals', goalProvider);

    vscode.commands.registerCommand('sentinel.refreshGoals', () => goalProvider.refresh());

    console.log('Sentinel extension and TreeView activated.');
}

class SentinelGoalProvider implements vscode.TreeDataProvider<GoalItem> {
    private _onDidChangeTreeData: vscode.EventEmitter<GoalItem | undefined | null | void> = new vscode.EventEmitter<GoalItem | undefined | null | void>();
    readonly onDidChangeTreeData: vscode.Event<GoalItem | undefined | null | void> = this._onDidChangeTreeData.event;

    constructor(private sentinelPath: string, private workspaceRoot: string) {}

    refresh(): void {
        this._onDidChangeTreeData.fire();
    }

    getTreeItem(element: GoalItem): vscode.TreeItem {
        return element;
    }

    async getChildren(element?: GoalItem): Promise<GoalItem[]> {
        if (element) {
            return [];
        } else {
            try {
                // Esegue il comando reale per ottenere i dati dal manifold
                const { stdout } = await execAsync(`"${this.sentinelPath}" status --json`, { cwd: this.workspaceRoot });
                const manifold = JSON.parse(stdout);
                
                if (manifold.error) {
                    return [new GoalItem("No manifold found", vscode.TreeItemCollapsibleState.None, "0%")];
                }

                // Esempio: Estraiamo l'intento principale e i primi goal dal DAG
                const goals: GoalItem[] = [];
                goals.push(new GoalItem(manifold.root_intent.description, vscode.TreeItemCollapsibleState.None, `${(manifold.goal_dag.nodes.length)} goals`));
                
                return goals;
            } catch (error) {
                console.error('Error fetching goals:', error);
                return [new GoalItem("Sentinel not found/init", vscode.TreeItemCollapsibleState.None, "Error")];
            }
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
