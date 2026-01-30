"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.MCPClient = void 0;
const events_1 = require("events");
const transport_1 = require("./transport");
const protocol_1 = require("./protocol");
const constants_1 = require("../shared/constants");
/**
 * MCP Client singleton that manages the connection to `sentinel mcp`.
 * Handles request/response correlation, reconnection, and convenience methods.
 */
class MCPClient extends events_1.EventEmitter {
    constructor(sentinelPath, cwd, outputChannel) {
        super();
        this.sentinelPath = sentinelPath;
        this.cwd = cwd;
        this.outputChannel = outputChannel;
        this.transport = null;
        this.nextId = 1;
        this.pendingRequests = new Map();
        this._initialized = false;
        this.reconnectAttempts = 0;
        this.reconnectTimer = null;
        this.disposed = false;
    }
    get connected() {
        return this.transport?.connected ?? false;
    }
    get initialized() {
        return this._initialized;
    }
    async start() {
        if (this.disposed)
            return;
        this.cancelReconnect();
        try {
            this.transport = new transport_1.StdioTransport(this.sentinelPath, ['mcp'], this.cwd);
            this.transport.on('message', (msg) => this.handleMessage(msg));
            this.transport.on('error', (err) => this.handleError(err));
            this.transport.on('close', (code) => this.handleClose(code));
            this.transport.on('stderr', (text) => {
                this.outputChannel.appendLine(`[sentinel-mcp stderr] ${text}`);
            });
            this.transport.start();
            // MCP initialization handshake
            const result = await this.request('initialize', {
                protocolVersion: '2024-11-05',
                capabilities: {},
                clientInfo: { name: 'sentinel-vscode', version: '2.0.0' },
            });
            this.outputChannel.appendLine(`MCP connected: ${result.serverInfo.name} v${result.serverInfo.version}`);
            // Send initialized notification
            this.notify('notifications/initialized', {});
            this._initialized = true;
            this.reconnectAttempts = 0;
            this.emit('connected');
        }
        catch (err) {
            this._initialized = false;
            const error = err instanceof Error ? err : new Error(String(err));
            this.outputChannel.appendLine(`MCP connection failed: ${error.message}`);
            this.scheduleReconnect();
            throw error;
        }
    }
    stop() {
        this.disposed = true;
        this.cancelReconnect();
        this._initialized = false;
        // Reject all pending requests
        for (const [, pending] of this.pendingRequests) {
            clearTimeout(pending.timer);
            pending.reject(new Error('MCP client stopped'));
        }
        this.pendingRequests.clear();
        this.transport?.stop();
        this.transport = null;
    }
    // ── Convenience methods ──────────────────────────────────
    async getAlignment() {
        return this.callTool('get_alignment', {});
    }
    async validateAction(actionType, description) {
        return this.callTool('validate_action', {
            action_type: actionType,
            description,
        });
    }
    async safeWrite(filePath, content) {
        return this.callTool('safe_write', {
            file_path: filePath,
            content,
        });
    }
    async getCognitiveMap() {
        return this.callTool('get_cognitive_map', {});
    }
    async getEnforcementRules() {
        const result = await this.callTool('get_enforcement_rules', {});
        return result;
    }
    async proposeStrategy(goalDescription) {
        return this.callTool('propose_strategy', {
            goal_description: goalDescription,
        });
    }
    async recordHandover(goalId, content, warnings) {
        await this.callTool('record_handover', {
            goal_id: goalId,
            content,
            warnings,
        });
    }
    async listTools() {
        return this.request('tools/list', {});
    }
    // ── Core protocol methods ────────────────────────────────
    async callTool(name, args) {
        const result = await this.request('tools/call', {
            name,
            arguments: args,
        });
        if (result.isError) {
            const errorText = result.content?.[0]?.text ?? 'Unknown MCP tool error';
            throw new Error(errorText);
        }
        const text = result.content?.[0]?.text;
        if (!text)
            return {};
        try {
            return JSON.parse(text);
        }
        catch {
            return { text };
        }
    }
    request(method, params) {
        return new Promise((resolve, reject) => {
            if (!this.transport?.connected) {
                reject(new Error('MCP not connected'));
                return;
            }
            const id = this.nextId++;
            const req = {
                jsonrpc: '2.0',
                method,
                params,
                id,
            };
            const timer = setTimeout(() => {
                this.pendingRequests.delete(id);
                reject(new Error(`MCP request timeout: ${method}`));
            }, 30000);
            this.pendingRequests.set(id, { resolve, reject, timer });
            this.transport.send(req);
        });
    }
    notify(method, params) {
        if (!this.transport?.connected)
            return;
        this.transport.send({ jsonrpc: '2.0', method, params });
    }
    // ── Message handling ─────────────────────────────────────
    handleMessage(msg) {
        if ((0, protocol_1.isResponse)(msg)) {
            const response = msg;
            const id = response.id;
            if (id === null)
                return;
            const pending = this.pendingRequests.get(id);
            if (!pending)
                return;
            this.pendingRequests.delete(id);
            clearTimeout(pending.timer);
            if (response.error) {
                pending.reject(new Error(response.error.message));
            }
            else {
                pending.resolve(response.result);
            }
        }
        else if ((0, protocol_1.isNotification)(msg)) {
            // Forward notifications (e.g. alignment updates, chat streaming)
            this.emit('notification', msg);
        }
    }
    handleError(err) {
        this.outputChannel.appendLine(`MCP error: ${err.message}`);
        this._initialized = false;
        this.emit('error', err);
    }
    handleClose(code) {
        this.outputChannel.appendLine(`MCP process exited with code ${code}`);
        this._initialized = false;
        this.emit('disconnected');
        if (!this.disposed) {
            this.scheduleReconnect();
        }
    }
    // ── Reconnection ─────────────────────────────────────────
    scheduleReconnect() {
        if (this.disposed || this.reconnectTimer)
            return;
        const delay = Math.min(constants_1.RECONNECT_BASE_MS * Math.pow(2, this.reconnectAttempts), constants_1.RECONNECT_MAX_MS);
        this.reconnectAttempts++;
        this.outputChannel.appendLine(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})...`);
        this.reconnectTimer = setTimeout(async () => {
            this.reconnectTimer = null;
            try {
                await this.start();
            }
            catch {
                // start() will schedule another reconnect on failure
            }
        }, delay);
    }
    cancelReconnect() {
        if (this.reconnectTimer) {
            clearTimeout(this.reconnectTimer);
            this.reconnectTimer = null;
        }
    }
}
exports.MCPClient = MCPClient;
//# sourceMappingURL=client.js.map