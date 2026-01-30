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
exports.NetworkProvider = void 0;
const vscode = __importStar(require("vscode"));
const child_process_1 = require("child_process");
class NetworkItem extends vscode.TreeItem {
    constructor(label, description, collapsible = vscode.TreeItemCollapsibleState.None) {
        super(label, collapsible);
        this.description = description;
    }
}
class NetworkProvider {
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
            const item = new NetworkItem('Sentinel not connected', 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }
        try {
            const report = await this.runStatusJson();
            const manifold = report?.manifold;
            if (!manifold)
                return [new NetworkItem('No manifold', '')];
            const items = [];
            const peerCount = manifold.peer_count ?? 0;
            const peerItem = new NetworkItem(`Connected Peers: ${peerCount}`, peerCount > 0 ? 'ONLINE' : 'SEARCHING');
            peerItem.iconPath = new vscode.ThemeIcon('broadcast');
            items.push(peerItem);
            const consensusItem = new NetworkItem(manifold.consensus_active ? 'Consensus: VOTING' : 'Global Consensus Stable', manifold.consensus_active ? 'ACTIVE' : 'SYNCED');
            consensusItem.iconPath = new vscode.ThemeIcon(manifold.consensus_active ? 'check-all' : 'shield');
            items.push(consensusItem);
            return items;
        }
        catch {
            return [new NetworkItem('Network Offline', 'OFFLINE')];
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
exports.NetworkProvider = NetworkProvider;
//# sourceMappingURL=networkProvider.js.map