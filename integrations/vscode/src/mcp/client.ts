import { EventEmitter } from "events";
import * as vscode from "vscode";
import { StdioTransport } from "./transport";
import {
  JsonRpcRequest,
  JsonRpcResponse,
  McpInitializeResult,
  McpToolCallParams,
  McpToolCallResult,
  McpToolsListResult,
  isNotification,
  isResponse,
} from "./protocol";
import { RECONNECT_BASE_MS, RECONNECT_MAX_MS } from "../shared/constants";
import type {
  AlignmentReport,
  ValidationResult,
  SecurityScanResult,
  CognitiveMap,
  EnforcementRule,
  StrategyRecommendation,
} from "../shared/types";

/**
 * MCP Client singleton that manages the connection to `sentinel mcp`.
 * Handles request/response correlation, reconnection, and convenience methods.
 */
export class MCPClient extends EventEmitter {
  private transport: StdioTransport | null = null;
  private nextId: number = 1;
  private pendingRequests: Map<
    number,
    {
      resolve: (value: unknown) => void;
      reject: (reason: Error) => void;
      timer: ReturnType<typeof setTimeout>;
    }
  > = new Map();
  private _initialized: boolean = false;
  private reconnectAttempts: number = 0;
  private reconnectTimer: ReturnType<typeof setTimeout> | null = null;
  private disposed: boolean = false;
  private envOverrides: NodeJS.ProcessEnv = {};

  constructor(
    private sentinelPath: string,
    private cwd: string,
    private outputChannel: vscode.OutputChannel,
    envOverrides: NodeJS.ProcessEnv = {},
  ) {
    super();
    this.envOverrides = envOverrides;
  }

  get connected(): boolean {
    return this.transport?.connected ?? false;
  }

  get initialized(): boolean {
    return this._initialized;
  }

  async start(): Promise<void> {
    if (this.disposed) return;
    this.cancelReconnect();

    try {
      this.transport = new StdioTransport(
        this.sentinelPath,
        ["mcp"],
        this.cwd,
        this.envOverrides,
      );

      this.transport.on("message", (msg: unknown) => this.handleMessage(msg));
      this.transport.on("error", (err: Error) => this.handleError(err));
      this.transport.on("close", (code: number) => this.handleClose(code));
      this.transport.on("stderr", (text: string) => {
        this.outputChannel.appendLine(`[sentinel-mcp stderr] ${text}`);
      });

      this.transport.start();

      // MCP initialization handshake
      const result = (await this.request("initialize", {
        protocolVersion: "2024-11-05",
        capabilities: {},
        clientInfo: { name: "sentinel-vscode", version: "2.0.0" },
      })) as McpInitializeResult;

      this.outputChannel.appendLine(
        `MCP connected: ${result.serverInfo.name} v${result.serverInfo.version}`,
      );

      // Send initialized notification
      this.notify("notifications/initialized", {});

      this._initialized = true;
      this.reconnectAttempts = 0;
      this.emit("connected");
    } catch (err) {
      this._initialized = false;
      const error = err instanceof Error ? err : new Error(String(err));
      this.outputChannel.appendLine(`MCP connection failed: ${error.message}`);
      this.scheduleReconnect();
      throw error;
    }
  }

  stop(): void {
    this.disposed = true;
    this.cancelReconnect();
    this._initialized = false;

    // Reject all pending requests
    for (const [, pending] of this.pendingRequests) {
      clearTimeout(pending.timer);
      pending.reject(new Error("MCP client stopped"));
    }
    this.pendingRequests.clear();

    this.transport?.stop();
    this.transport = null;
  }

  setEnvOverrides(env: NodeJS.ProcessEnv): void {
    this.envOverrides = env;
  }

  disconnect(): void {
    this.cancelReconnect();
    this._initialized = false;
    this.transport?.stop();
    this.transport = null;
    this.emit("disconnected");
  }

  // ── Convenience methods ──────────────────────────────────

  async getAlignment(): Promise<AlignmentReport> {
    return this.callTool("get_alignment", {}) as Promise<AlignmentReport>;
  }

  async validateAction(
    actionType: string,
    description: string,
  ): Promise<ValidationResult> {
    return this.callTool("validate_action", {
      action_type: actionType,
      description,
    }) as Promise<ValidationResult>;
  }

  async safeWrite(
    filePath: string,
    content: string,
  ): Promise<SecurityScanResult> {
    return this.callTool("safe_write", {
      file_path: filePath,
      content,
    }) as Promise<SecurityScanResult>;
  }

  async getCognitiveMap(): Promise<CognitiveMap> {
    return this.callTool("get_cognitive_map", {}) as Promise<CognitiveMap>;
  }

  async getEnforcementRules(): Promise<EnforcementRule[]> {
    const result = await this.callTool("get_enforcement_rules", {});
    return result as EnforcementRule[];
  }

  async proposeStrategy(
    goalDescription: string,
  ): Promise<StrategyRecommendation> {
    return this.callTool("propose_strategy", {
      goal_description: goalDescription,
    }) as Promise<StrategyRecommendation>;
  }

  async recordHandover(
    goalId: string,
    content: string,
    warnings: string[],
  ): Promise<void> {
    await this.callTool("record_handover", {
      goal_id: goalId,
      content,
      warnings,
    });
  }

  async listTools(): Promise<McpToolsListResult> {
    return this.request("tools/list", {}) as Promise<McpToolsListResult>;
  }

  // ── Core protocol methods ────────────────────────────────

  async callTool(
    name: string,
    args: Record<string, unknown>,
  ): Promise<unknown> {
    const result = (await this.request("tools/call", {
      name,
      arguments: args,
    } as Record<string, unknown>)) as McpToolCallResult;

    if (result.isError) {
      const errorText = result.content?.[0]?.text ?? "Unknown MCP tool error";
      throw new Error(errorText);
    }

    const text = result.content?.[0]?.text;
    if (!text) return {};

    try {
      return JSON.parse(text);
    } catch {
      return { text };
    }
  }

  private request(
    method: string,
    params: Record<string, unknown>,
  ): Promise<unknown> {
    return new Promise((resolve, reject) => {
      if (!this.transport?.connected) {
        reject(new Error("MCP not connected"));
        return;
      }

      const id = this.nextId++;
      const req: JsonRpcRequest = {
        jsonrpc: "2.0",
        method,
        params,
        id,
      };

      const timer = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`MCP request timeout: ${method}`));
      }, 30_000);

      this.pendingRequests.set(id, { resolve, reject, timer });
      this.transport.send(req);
    });
  }

  private notify(method: string, params: Record<string, unknown>): void {
    if (!this.transport?.connected) return;
    this.transport.send({ jsonrpc: "2.0", method, params });
  }

  // ── Message handling ─────────────────────────────────────

  private handleMessage(msg: unknown): void {
    if (isResponse(msg)) {
      const response = msg as JsonRpcResponse;
      const id = response.id;
      if (id === null) return;

      const pending = this.pendingRequests.get(id);
      if (!pending) return;

      this.pendingRequests.delete(id);
      clearTimeout(pending.timer);

      if (response.error) {
        pending.reject(new Error(response.error.message));
      } else {
        pending.resolve(response.result);
      }
    } else if (isNotification(msg)) {
      // Forward notifications (e.g. alignment updates, chat streaming)
      this.emit("notification", msg);
    }
  }

  private handleError(err: Error): void {
    this.outputChannel.appendLine(`MCP error: ${err.message}`);
    this._initialized = false;
    this.emit("error", err);
  }

  private handleClose(code: number): void {
    this.outputChannel.appendLine(`MCP process exited with code ${code}`);
    this._initialized = false;
    this.emit("disconnected");

    if (!this.disposed) {
      this.scheduleReconnect();
    }
  }

  // ── Reconnection ─────────────────────────────────────────

  private scheduleReconnect(): void {
    if (this.disposed || this.reconnectTimer) return;

    const delay = Math.min(
      RECONNECT_BASE_MS * Math.pow(2, this.reconnectAttempts),
      RECONNECT_MAX_MS,
    );
    this.reconnectAttempts++;

    this.outputChannel.appendLine(
      `Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})...`,
    );

    this.reconnectTimer = setTimeout(async () => {
      this.reconnectTimer = null;
      try {
        await this.start();
      } catch {
        // start() will schedule another reconnect on failure
      }
    }, delay);
  }

  private cancelReconnect(): void {
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = null;
    }
  }
}
