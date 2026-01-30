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
exports.SentinelCodeLensProvider = void 0;
const vscode = __importStar(require("vscode"));
const constants_1 = require("../shared/constants");
class SentinelCodeLensProvider {
    constructor(client) {
        this.client = client;
        this._onDidChange = new vscode.EventEmitter();
        this.onDidChangeCodeLenses = this._onDidChange.event;
        this.lastReport = null;
    }
    updateReport(report) {
        this.lastReport = report;
        this._onDidChange.fire();
    }
    provideCodeLenses(document) {
        const range = new vscode.Range(0, 0, 0, 0);
        if (!this.client.connected) {
            return [
                new vscode.CodeLens(range, {
                    title: '$(shield) Sentinel: Not connected',
                    command: constants_1.CMD_SHOW_ALIGNMENT,
                }),
            ];
        }
        if (!this.lastReport) {
            return [
                new vscode.CodeLens(range, {
                    title: '$(shield) Sentinel Alignment: ...',
                    command: constants_1.CMD_SHOW_ALIGNMENT,
                }),
            ];
        }
        const score = this.lastReport.score.toFixed(0);
        return [
            new vscode.CodeLens(range, {
                title: `$(shield) Sentinel Alignment: ${score}%`,
                command: constants_1.CMD_SHOW_ALIGNMENT,
            }),
        ];
    }
}
exports.SentinelCodeLensProvider = SentinelCodeLensProvider;
//# sourceMappingURL=codeLensProvider.js.map