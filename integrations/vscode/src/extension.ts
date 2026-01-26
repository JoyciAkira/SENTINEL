import * as vscode from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions } from 'vscode-languageclient/node';
import { spawn } from 'child_process';

let client: LanguageClient;

export function activate(context: vscode.ExtensionContext) {
    let sentinelPath = vscode.workspace.getConfiguration('sentinel').get<string>('path') || 'sentinel';
    sentinelPath = sentinelPath.replace(/^["'](.+)["']$/, '$1');

    const workspaceRoot = vscode.workspace.workspaceFolders 
        ? vscode.workspace.workspaceFolders[0].uri.fsPath 
        : ".";

    // 1. Setup LSP Client
    const serverOptions: ServerOptions = {
        command: sentinelPath,
        args: ['lsp'],
        options: { cwd: workspaceRoot }
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
    const goalProvider = new SentinelGoalProvider(sentinelPath, workspaceRoot);
    vscode.window.registerTreeDataProvider('sentinel-goals', goalProvider);

    // 3. Setup Network TreeView
    const networkProvider = new SentinelNetworkProvider(sentinelPath, workspaceRoot);
    vscode.window.registerTreeDataProvider('sentinel-network', networkProvider);

    vscode.commands.registerCommand('sentinel.refreshGoals', () => {
        goalProvider.refresh();
        networkProvider.refresh();
    });

    console.log('Sentinel extension activated. Path:', sentinelPath);
}

function runSentinelJson(sentinelPath: string, args: string[], cwd: string): Promise<any> {
    return new Promise((resolve, reject) => {
        const proc = spawn(sentinelPath, args, { cwd, shell: false });
        let stdout = '';
        let stderr = '';

        proc.stdout.on('data', (data) => stdout += data.toString());
        proc.stderr.on('data', (data) => stderr += data.toString());

        proc.on('error', (err) => reject(new Error(`Spawn error: ${err.message}`)));

        proc.on('close', (code) => {
            if (code === 0) {
                try {
                    resolve(JSON.parse(stdout));
                } catch (e) {
                    reject(new Error(`JSON Parse error. Output was: ${stdout}`));
                }
            } else {
                reject(new Error(`Exit code ${code}. Stderr: ${stderr}`));
            }
        });
    });
}

class GoalItem extends vscode.TreeItem {
    public children?: GoalItem[];

    constructor(
        public readonly label: string,
        public readonly collapsibleState: vscode.TreeItemCollapsibleState,
        public readonly progress: string
    ) {
        super(label, collapsibleState);
        this.tooltip = `${this.label} - ${this.progress}`;
        this.description = this.progress;
    }
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
            return element.children || [];
        } else {
            try {
                const report = await runSentinelJson(this.sentinelPath, ["status", "--json"], this.workspaceRoot);
                
                if (!report.manifold || report.manifold.error) {
                    return [new GoalItem("Manifold non inizializzato", vscode.TreeItemCollapsibleState.None, "Esegui 'sentinel init'")];
                }

                const manifold = report.manifold;
                const external = report.external;
                const rootItems: GoalItem[] = [];
                
                // 1. Goal Section
                const goalRoot = new GoalItem("Goal Manifold", vscode.TreeItemCollapsibleState.Expanded, "CORE");
                goalRoot.iconPath = new vscode.ThemeIcon('target');
                goalRoot.children = [];
                goalRoot.children.push(new GoalItem(manifold.root_intent.description, vscode.TreeItemCollapsibleState.None, "ROOT"));
                
                if (manifold.goal_dag && manifold.goal_dag.nodes) {
                    Object.values(manifold.goal_dag.nodes).forEach((node: any) => {
                        goalRoot.children?.push(new GoalItem(node.description, vscode.TreeItemCollapsibleState.None, node.status));
                    });
                }
                rootItems.push(goalRoot);

                // 2. Multi-Agent Section (Social Manifold)
                const agentsRoot = new GoalItem("Active Agents", vscode.TreeItemCollapsibleState.Collapsed, "HERD");
                agentsRoot.iconPath = new vscode.ThemeIcon('organization');
                agentsRoot.children = [];
                
                if (manifold.file_locks && Object.keys(manifold.file_locks).length > 0) {
                    for (const file in manifold.file_locks) {
                        const agentId = manifold.file_locks[file].substring(0, 8);
                        const item = new GoalItem(`Agent ${agentId}`, vscode.TreeItemCollapsibleState.None, `Working on ${file.split('/').pop()}`);
                        item.iconPath = new vscode.ThemeIcon('account');
                        agentsRoot.children.push(item);
                    }
                } else {
                    agentsRoot.children.push(new GoalItem("No active agents", vscode.TreeItemCollapsibleState.None, "IDLE"));
                }
                rootItems.push(agentsRoot);

                // 3. Cognitive Trail (Handover Notes)
                const trailRoot = new GoalItem("Cognitive Trail", vscode.TreeItemCollapsibleState.Collapsed, "MEMORY");
                trailRoot.iconPath = new vscode.ThemeIcon('history');
                trailRoot.children = [];
                
                if (manifold.handover_log && manifold.handover_log.length > 0) {
                    manifold.handover_log.slice(-5).reverse().forEach((note: any) => {
                        const item = new GoalItem(note.content, vscode.TreeItemCollapsibleState.None, "NOTE");
                        item.iconPath = new vscode.ThemeIcon('note');
                        trailRoot.children?.push(item);
                    });
                } else {
                    trailRoot.children.push(new GoalItem("No handover notes", vscode.TreeItemCollapsibleState.None, "EMPTY"));
                }
                rootItems.push(trailRoot);

                // 4. External Awareness Section
                const riskLevel = external ? Math.round(external.risk_level * 100) : 0;
                const externalRoot = new GoalItem("External Awareness", vscode.TreeItemCollapsibleState.Collapsed, `Risk: ${riskLevel}%`);
                externalRoot.iconPath = new vscode.ThemeIcon('globe');
                externalRoot.children = [];
                
                if (external && external.alerts && external.alerts.length > 0) {
                    external.alerts.forEach((alert: string) => {
                        const item = new GoalItem(alert, vscode.TreeItemCollapsibleState.None, "ALERT");
                        item.iconPath = new vscode.ThemeIcon('warning', new vscode.ThemeColor('notificationsWarningIcon.foreground'));
                        externalRoot.children?.push(item);
                    });
                } else {
                    externalRoot.children.push(new GoalItem("No threats detected", vscode.TreeItemCollapsibleState.None, "SECURE"));
                }
                rootItems.push(externalRoot);

                // 5. Distributed Intelligence (Phase 3)
                const networkRoot = new GoalItem("Sentinel Network", vscode.TreeItemCollapsibleState.Collapsed, "P2P");
                networkRoot.iconPath = new vscode.ThemeIcon('hubot');
                networkRoot.children = [];
                
                // Questi dati verranno popolati dal comando status --json aggiornato con i dati del Layer 9/10
                const peerCount = manifold.peer_count || 0;
                networkRoot.children.push(new GoalItem(`Connected Peers: ${peerCount}`, vscode.TreeItemCollapsibleState.None, peerCount > 0 ? "ONLINE" : "SEARCHING"));
                
                if (manifold.consensus_active) {
                    const item = new GoalItem("Consensus Vote in Progress", vscode.TreeItemCollapsibleState.None, "VOTING");
                    item.iconPath = new vscode.ThemeIcon('check-all');
                    networkRoot.children.push(item);
                } else {
                    networkRoot.children.push(new GoalItem("Global Consensus Stable", vscode.TreeItemCollapsibleState.None, "SYNCED"));
                }
                rootItems.push(networkRoot);
                
                return rootItems;
            } catch (error: any) {
                return [new GoalItem("Errore Sentinel", vscode.TreeItemCollapsibleState.None, error.message)];
            }
        }
    }
}

class SentinelNetworkProvider implements vscode.TreeDataProvider<GoalItem> {
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
        if (element) return [];
        
        try {
            const report = await runSentinelJson(this.sentinelPath, ["status", "--json"], this.workspaceRoot);
            const manifold = report.manifold;
            
            const peerCount = manifold.peer_count || 0;
            const peerItem = new GoalItem(`Connected Peers: ${peerCount}`, vscode.TreeItemCollapsibleState.None, peerCount > 0 ? "ONLINE" : "SEARCHING");
            peerItem.iconPath = new vscode.ThemeIcon('broadcast');

            const consensusItem = new GoalItem(
                manifold.consensus_active ? "Consensus: VOTING" : "Global Consensus Stable", 
                vscode.TreeItemCollapsibleState.None, 
                manifold.consensus_active ? "ACTIVE" : "SYNCED"
            );
            consensusItem.iconPath = new vscode.ThemeIcon(manifold.consensus_active ? 'check-all' : 'shield');

            return [peerItem, consensusItem];
        } catch (e) {
            return [new GoalItem("Network Offline", vscode.TreeItemCollapsibleState.None, "OFFLINE")];
        }
    }
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}