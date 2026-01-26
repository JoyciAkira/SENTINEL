"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || function (mod) {
    if (mod && mod.__esModule) return mod;
    var result = {};
    if (mod != null) for (var k in mod) if (k !== "default" && Object.prototype.hasOwnProperty.call(mod, k)) __createBinding(result, mod, k);
    __setModuleDefault(result, mod);
    return result;
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.deactivate = exports.activate = void 0;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
const child_process_1 = require("child_process");
let client;
function activate(context) {
    let sentinelPath = vscode.workspace.getConfiguration('sentinel').get('path') || 'sentinel';
    sentinelPath = sentinelPath.replace(/^["'](.+)["']$/, '$1');
    const workspaceRoot = vscode.workspace.workspaceFolders
        ? vscode.workspace.workspaceFolders[0].uri.fsPath
        : ".";
    // 1. Setup LSP Client
    const serverOptions = {
        command: sentinelPath,
        args: ['lsp'],
        options: { cwd: workspaceRoot }
    };
    const clientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'rust' },
            { scheme: 'file', language: 'typescript' },
            { scheme: 'file', language: 'javascript' },
            { scheme: 'file', language: 'python' }
        ]
    };
    client = new node_1.LanguageClient('sentinelLSP', 'Sentinel LSP', serverOptions, clientOptions);
    client.start();
    // 2. Setup Goal TreeView
    const goalProvider = new SentinelGoalProvider(sentinelPath, workspaceRoot);
    vscode.window.registerTreeDataProvider('sentinel-goals', goalProvider);
    vscode.commands.registerCommand('sentinel.refreshGoals', () => goalProvider.refresh());
    console.log('Sentinel extension activated. Path:', sentinelPath);
}
exports.activate = activate;
function runSentinelJson(sentinelPath, args, cwd) {
    return new Promise((resolve, reject) => {
        const proc = (0, child_process_1.spawn)(sentinelPath, args, { cwd, shell: false });
        let stdout = '';
        let stderr = '';
        proc.stdout.on('data', (data) => stdout += data.toString());
        proc.stderr.on('data', (data) => stderr += data.toString());
        proc.on('error', (err) => reject(new Error(`Spawn error: ${err.message}`)));
        proc.on('close', (code) => {
            if (code === 0) {
                try {
                    resolve(JSON.parse(stdout));
                }
                catch (e) {
                    reject(new Error(`JSON Parse error. Output was: ${stdout}`));
                }
            }
            else {
                reject(new Error(`Exit code ${code}. Stderr: ${stderr}`));
            }
        });
    });
}
class GoalItem extends vscode.TreeItem {
    constructor(label, collapsibleState, progress) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        this.progress = progress;
        this.tooltip = `${this.label} - ${this.progress}`;
        this.description = this.progress;
    }
}
class SentinelGoalProvider {
    constructor(sentinelPath, workspaceRoot) {
        this.sentinelPath = sentinelPath;
        this.workspaceRoot = workspaceRoot;
        this._onDidChangeTreeData = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChangeTreeData.event;
    }
    refresh() {
        this._onDidChangeTreeData.fire();
    }
    getTreeItem(element) {
        return element;
    }
    async getChildren(element) {
        if (element) {
            return element.children || [];
        }
        else {
            try {
                const report = await runSentinelJson(this.sentinelPath, ["status", "--json"], this.workspaceRoot);
                if (!report.manifold || report.manifold.error) {
                    return [new GoalItem("Manifold non inizializzato", vscode.TreeItemCollapsibleState.None, "Esegui 'sentinel init'")];
                }
                const manifold = report.manifold;
                const external = report.external;
                const rootItems = [];
                // 1. Goal Section
                const goalRoot = new GoalItem("Goal Manifold", vscode.TreeItemCollapsibleState.Expanded, "CORE");
                goalRoot.iconPath = new vscode.ThemeIcon('target');
                goalRoot.children = [];
                goalRoot.children.push(new GoalItem(manifold.root_intent.description, vscode.TreeItemCollapsibleState.None, "ROOT"));
                if (manifold.goal_dag && manifold.goal_dag.nodes) {
                    Object.values(manifold.goal_dag.nodes).forEach((node) => {
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
                }
                else {
                    agentsRoot.children.push(new GoalItem("No active agents", vscode.TreeItemCollapsibleState.None, "IDLE"));
                }
                rootItems.push(agentsRoot);
                // 3. Cognitive Trail (Handover Notes)
                const trailRoot = new GoalItem("Cognitive Trail", vscode.TreeItemCollapsibleState.Collapsed, "MEMORY");
                trailRoot.iconPath = new vscode.ThemeIcon('history');
                trailRoot.children = [];
                if (manifold.handover_log && manifold.handover_log.length > 0) {
                    manifold.handover_log.slice(-5).reverse().forEach((note) => {
                        const item = new GoalItem(note.content, vscode.TreeItemCollapsibleState.None, "NOTE");
                        item.iconPath = new vscode.ThemeIcon('note');
                        trailRoot.children?.push(item);
                    });
                }
                else {
                    trailRoot.children.push(new GoalItem("No handover notes", vscode.TreeItemCollapsibleState.None, "EMPTY"));
                }
                rootItems.push(trailRoot);
                // 4. External Awareness Section
                const riskLevel = external ? Math.round(external.risk_level * 100) : 0;
                const externalRoot = new GoalItem("External Awareness", vscode.TreeItemCollapsibleState.Collapsed, `Risk: ${riskLevel}%`);
                externalRoot.iconPath = new vscode.ThemeIcon('globe');
                externalRoot.children = [];
                if (external && external.alerts && external.alerts.length > 0) {
                    external.alerts.forEach((alert) => {
                        const item = new GoalItem(alert, vscode.TreeItemCollapsibleState.None, "ALERT");
                        item.iconPath = new vscode.ThemeIcon('warning', new vscode.ThemeColor('notificationsWarningIcon.foreground'));
                        externalRoot.children?.push(item);
                    });
                }
                else {
                    externalRoot.children.push(new GoalItem("No threats detected", vscode.TreeItemCollapsibleState.None, "SECURE"));
                }
                rootItems.push(externalRoot);
                return rootItems;
            }
            catch (error) {
                return [new GoalItem("Errore Sentinel", vscode.TreeItemCollapsibleState.None, error.message)];
            }
        }
    }
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
exports.deactivate = deactivate;
//# sourceMappingURL=extension.js.map