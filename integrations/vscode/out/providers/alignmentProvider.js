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
exports.AlignmentProvider = void 0;
const vscode = __importStar(require("vscode"));
const constants_1 = require("../shared/constants");
class AlignmentItem extends vscode.TreeItem {
    constructor(label, description, collapsible = vscode.TreeItemCollapsibleState.None, children) {
        super(label, collapsible);
        this.children = children;
        this.description = description;
    }
}
class AlignmentProvider {
    constructor(client) {
        this.client = client;
        this._onDidChange = new vscode.EventEmitter();
        this.onDidChangeTreeData = this._onDidChange.event;
        this.lastReport = null;
    }
    refresh() {
        this._onDidChange.fire();
    }
    updateReport(report) {
        this.lastReport = report;
        this.refresh();
    }
    getTreeItem(element) {
        return element;
    }
    async getChildren(element) {
        if (element)
            return element.children ?? [];
        if (!this.client.connected) {
            const item = new AlignmentItem('Sentinel not connected', 'Install sentinel-cli');
            item.iconPath = new vscode.ThemeIcon('debug-disconnect');
            return [item];
        }
        try {
            if (!this.lastReport) {
                this.lastReport = await this.client.getAlignment();
            }
            const r = this.lastReport;
            const scoreItem = new AlignmentItem(`Score: ${r.score.toFixed(1)}%`, r.status);
            scoreItem.iconPath = new vscode.ThemeIcon(r.score >= constants_1.ALIGNMENT_EXCELLENT ? 'pass-filled' :
                r.score >= constants_1.ALIGNMENT_GOOD ? 'pass' :
                    r.score >= constants_1.ALIGNMENT_CONCERNING ? 'warning' : 'error', r.score >= constants_1.ALIGNMENT_GOOD
                ? new vscode.ThemeColor('testing.iconPassed')
                : r.score >= constants_1.ALIGNMENT_CONCERNING
                    ? new vscode.ThemeColor('notificationsWarningIcon.foreground')
                    : new vscode.ThemeColor('testing.iconFailed'));
            const confItem = new AlignmentItem(`Confidence: ${(r.confidence * 100).toFixed(0)}%`, '');
            confItem.iconPath = new vscode.ThemeIcon('graph');
            const items = [scoreItem, confItem];
            if (r.violations.length > 0) {
                const violationChildren = r.violations.map(v => {
                    const vi = new AlignmentItem(v.description, `Severity: ${v.severity}`);
                    vi.iconPath = new vscode.ThemeIcon('warning', new vscode.ThemeColor('notificationsWarningIcon.foreground'));
                    return vi;
                });
                const violRoot = new AlignmentItem(`Violations (${r.violations.length})`, '', vscode.TreeItemCollapsibleState.Expanded, violationChildren);
                violRoot.iconPath = new vscode.ThemeIcon('issues');
                items.push(violRoot);
            }
            return items;
        }
        catch {
            return [new AlignmentItem('Failed to load alignment', 'Retry')];
        }
    }
}
exports.AlignmentProvider = AlignmentProvider;
//# sourceMappingURL=alignmentProvider.js.map