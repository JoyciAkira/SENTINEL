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
exports.SecurityProvider = void 0;
const vscode = __importStar(require("vscode"));
const child_process_1 = require("child_process");
class SecurityItem extends vscode.TreeItem {
    constructor(label, description, collapsible = vscode.TreeItemCollapsibleState.None) {
        super(label, collapsible);
        this.description = description;
    }
}
class SecurityProvider {
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
            return [];
        if (!this.client.connected) {
            const item = new SecurityItem('Sentinel not connected', 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }
        try {
            const report = await this.runStatusJson();
            const external = report?.external;
            const items = [];
            const riskLevel = external ? Math.round(external.risk_level * 100) : 0;
            const riskItem = new SecurityItem(`Risk Level: ${riskLevel}%`, riskLevel === 0 ? 'SECURE' : 'AT RISK');
            riskItem.iconPath = new vscode.ThemeIcon(riskLevel === 0 ? 'shield' : 'warning', riskLevel === 0
                ? new vscode.ThemeColor('testing.iconPassed')
                : new vscode.ThemeColor('notificationsWarningIcon.foreground'));
            items.push(riskItem);
            if (external?.alerts && external.alerts.length > 0) {
                for (const alert of external.alerts) {
                    const alertItem = new SecurityItem(alert, 'ALERT');
                    alertItem.iconPath = new vscode.ThemeIcon('warning', new vscode.ThemeColor('notificationsWarningIcon.foreground'));
                    items.push(alertItem);
                }
            }
            else {
                const safeItem = new SecurityItem('No threats detected', 'SECURE');
                safeItem.iconPath = new vscode.ThemeIcon('pass-filled', new vscode.ThemeColor('testing.iconPassed'));
                items.push(safeItem);
            }
            return items;
        }
        catch {
            return [new SecurityItem('Error loading security', '')];
        }
    }
    runStatusJson() {
        return new Promise((resolve, reject) => {
            const proc = (0, child_process_1.spawn)(this.sentinelPath, ['status', '--json'], {
                cwd: this.cwd,
                shell: false,
            });
            let stdout = '';
            proc.stdout.on('data', (d) => (stdout += d.toString()));
            proc.on('error', (err) => reject(err));
            proc.on('close', (code) => {
                if (code === 0) {
                    try {
                        resolve(JSON.parse(stdout));
                    }
                    catch {
                        reject(new Error('Parse error'));
                    }
                }
                else {
                    reject(new Error(`Exit ${code}`));
                }
            });
        });
    }
}
exports.SecurityProvider = SecurityProvider;
//# sourceMappingURL=securityProvider.js.map