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
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.GoalTreeProvider = void 0;
const vscode = __importStar(require("vscode"));
const child_process_1 = require("child_process");
class GoalItem extends vscode.TreeItem {
    constructor(label, collapsible, description = '') {
        super(label, collapsible);
        this.description = description;
        this.tooltip = `${label} - ${description}`;
    }
}
class GoalTreeProvider {
    constructor(client, sentinelPath, cwd) {
        this.client = client;
        this.sentinelPath = sentinelPath;
        this.cwd = cwd;
        this._onDidChange = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChange.event;
    }
    refresh() {
        this._onDidChange.fire();
    }
    getTreeItem(element) {
        return element;
    }
    async getChildren(element) {
        if (element)
            return element.children ?? [];
        if (!this.client.connected) {
            const item = new GoalItem('Sentinel not connected', vscode.TreeItemCollapsibleState.None, 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }
        try {
            const report = await this.runStatusJson();
            if (!report?.manifold || report.manifold.error) {
                return [new GoalItem('Manifold not initialized', vscode.TreeItemCollapsibleState.None, "Run 'sentinel init'")];
            }
            const manifold = report.manifold;
            const items = [];
            // Root intent
            const rootItem = new GoalItem(manifold.root_intent.description, vscode.TreeItemCollapsibleState.None, 'ROOT');
            rootItem.iconPath = new vscode.ThemeIcon('target');
            items.push(rootItem);
            // Goal DAG nodes
            if (manifold.goal_dag?.nodes) {
                const nodes = Object.values(manifold.goal_dag.nodes);
                const total = nodes.length;
                const completed = nodes.filter(n => n.status === 'Completed').length;
                for (const node of nodes) {
                    const goalItem = new GoalItem(node.description, vscode.TreeItemCollapsibleState.None, node.status);
                    goalItem.iconPath = new vscode.ThemeIcon(statusIcon(node.status));
                    items.push(goalItem);
                }
                // Summary header
                if (total > 0) {
                    const summaryItem = new GoalItem(`Progress: ${completed}/${total}`, vscode.TreeItemCollapsibleState.None, `${Math.round((completed / total) * 100)}%`);
                    summaryItem.iconPath = new vscode.ThemeIcon('pie-chart');
                    items.unshift(summaryItem);
                }
            }
            return items;
        }
        catch (err) {
            return [new GoalItem('Error loading goals', vscode.TreeItemCollapsibleState.None, err.message)];
        }
    }
    runStatusJson() {
        return new Promise((resolve, reject) => {
            const proc = (0, child_process_1.spawn)(this.sentinelPath, ['status', '--json'], {
                cwd: this.cwd,
                shell: false,
            });
            let stdout = '';
            let stderr = '';
            proc.stdout.on('data', (d) => (stdout += d.toString()));
            proc.stderr.on('data', (d) => (stderr += d.toString()));
            proc.on('error', (err) => reject(err));
            proc.on('close', (code) => {
                if (code === 0) {
                    try {
                        resolve(JSON.parse(stdout));
                    }
                    catch (e) {
                        reject(new Error(`JSON parse error: ${stdout}`));
                    }
                }
                else {
                    reject(new Error(`Exit code ${code}: ${stderr}`));
                }
            });
        });
    }
}
exports.GoalTreeProvider = GoalTreeProvider;
function statusIcon(status) {
    switch (status) {
        case 'Completed': return 'pass-filled';
        case 'InProgress': return 'sync~spin';
        case 'Validating': return 'beaker';
        case 'Ready': return 'circle-outline';
        case 'Pending': return 'circle-outline';
        case 'Blocked': return 'lock';
        case 'Failed': return 'error';
        case 'Deprecated': return 'trash';
        default: return 'question';
    }
}
//# sourceMappingURL=goalTreeProvider.js.map