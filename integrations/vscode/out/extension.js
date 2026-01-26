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
const util_1 = require("util");
const execAsync = (0, util_1.promisify)(child_process_1.exec);
let client;
function activate(context) {
    const sentinelPath = vscode.workspace.getConfiguration('sentinel').get('path') || 'sentinel';
    // 1. Setup LSP Client
    const serverOptions = {
        command: sentinelPath,
        args: ['lsp'],
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
    const goalProvider = new SentinelGoalProvider(sentinelPath, vscode.workspace.rootPath || ".");
    vscode.window.registerTreeDataProvider('sentinel-goals', goalProvider);
    vscode.commands.registerCommand('sentinel.refreshGoals', () => goalProvider.refresh());
    console.log('Sentinel extension and TreeView activated.');
}
exports.activate = activate;
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
                const { stdout } = await execAsync(`"${this.sentinelPath}" status --json`, {
                    cwd: this.workspaceRoot,
                    env: { ...process.env }
                });
                const report = JSON.parse(stdout);
                const manifold = report.manifold;
                const external = report.external;
                if (manifold.error) {
                    return [new GoalItem("Manifold non inizializzato", vscode.TreeItemCollapsibleState.None, "Usa 'sentinel init'")];
                }
                const rootItems = [];
                // 1. Goal Section
                const goalRoot = new GoalItem("Goal Manifold", vscode.TreeItemCollapsibleState.Expanded, "CORE");
                goalRoot.children = [];
                goalRoot.children.push(new GoalItem(manifold.root_intent.description, vscode.TreeItemCollapsibleState.None, "ROOT"));
                if (manifold.goal_dag && manifold.goal_dag.nodes) {
                    for (const node_id in manifold.goal_dag.nodes) {
                        const node = manifold.goal_dag.nodes[node_id];
                        goalRoot.children.push(new GoalItem(node.description, vscode.TreeItemCollapsibleState.None, node.status));
                    }
                }
                rootItems.push(goalRoot);
                // 2. External Awareness Section
                const externalRoot = new GoalItem("External Awareness", vscode.TreeItemCollapsibleState.Expanded, `Risk: ${Math.round(external.risk_level * 100)}%`);
                externalRoot.iconPath = new vscode.ThemeIcon('globe');
                externalRoot.children = [];
                if (external.alerts && external.alerts.length > 0) {
                    for (const alert of external.alerts) {
                        const item = new GoalItem(alert, vscode.TreeItemCollapsibleState.None, "ALERT");
                        item.iconPath = new vscode.ThemeIcon('warning', new vscode.ThemeColor('notificationsWarningIcon.foreground'));
                        externalRoot.children.push(item);
                    }
                }
                else {
                    externalRoot.children.push(new GoalItem("No threats detected", vscode.TreeItemCollapsibleState.None, "SECURE"));
                }
                rootItems.push(externalRoot);
                return rootItems;
            }
            catch (error) {
                console.error('Sentinel CLI Error:', error);
                return [new GoalItem("Errore Esecuzione CLI", vscode.TreeItemCollapsibleState.None, "Controlla il path")];
            }
        }
    }
}
class GoalItem extends vscode.TreeItem {
    constructor(label, collapsibleState, progress) {
        super(label, collapsibleState);
        this.label = label;
        this.collapsibleState = collapsibleState;
        this.progress = progress;
        this.tooltip = `${this.label} - ${this.progress}`;
        this.description = this.progress;
        this.iconPath = new vscode.ThemeIcon('target');
    }
}
function deactivate() {
    return client ? client.stop() : undefined;
}
exports.deactivate = deactivate;
//# sourceMappingURL=extension.js.map