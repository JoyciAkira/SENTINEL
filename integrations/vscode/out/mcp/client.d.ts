import { EventEmitter } from 'events';
import * as vscode from 'vscode';
import { McpToolsListResult } from './protocol';
import type { AlignmentReport, ValidationResult, SecurityScanResult, CognitiveMap, EnforcementRule, StrategyRecommendation } from '../shared/types';
/**
 * MCP Client singleton that manages the connection to `sentinel mcp`.
 * Handles request/response correlation, reconnection, and convenience methods.
 */
export declare class MCPClient extends EventEmitter {
    private sentinelPath;
    private cwd;
    private outputChannel;
    private transport;
    private nextId;
    private pendingRequests;
    private _initialized;
    private reconnectAttempts;
    private reconnectTimer;
    private disposed;
    constructor(sentinelPath: string, cwd: string, outputChannel: vscode.OutputChannel);
    get connected(): boolean;
    get initialized(): boolean;
    start(): Promise<void>;
    stop(): void;
    getAlignment(): Promise<AlignmentReport>;
    validateAction(actionType: string, description: string): Promise<ValidationResult>;
    safeWrite(filePath: string, content: string): Promise<SecurityScanResult>;
    getCognitiveMap(): Promise<CognitiveMap>;
    getEnforcementRules(): Promise<EnforcementRule[]>;
    proposeStrategy(goalDescription: string): Promise<StrategyRecommendation>;
    recordHandover(goalId: string, content: string, warnings: string[]): Promise<void>;
    listTools(): Promise<McpToolsListResult>;
    callTool(name: string, args: Record<string, unknown>): Promise<unknown>;
    private request;
    private notify;
    private handleMessage;
    private handleError;
    private handleClose;
    private scheduleReconnect;
    private cancelReconnect;
}
