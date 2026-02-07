import * as vscode from "vscode";
import { MCPClient } from "../mcp/client";
import { getWebviewContent } from "./getWebviewContent";
import type { AlignmentReport } from "../shared/types";
import { CMD_CODEX_LOGIN } from "../shared/constants";

/**
 * WebviewViewProvider for the Sentinel Chat sidebar panel.
 * Implements the Cline-style full sidebar chat experience.
 */
export class SentinelChatViewProvider implements vscode.WebviewViewProvider {
  public static readonly viewId = "sentinel-chat";

  private view?: vscode.WebviewView;
  private activeStreamId: string | null = null;

  constructor(
    private extensionUri: vscode.Uri,
    private client: MCPClient,
    private outputChannel: vscode.OutputChannel,
  ) {}

  resolveWebviewView(
    webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken,
  ): void {
    this.view = webviewView;

    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [
        vscode.Uri.joinPath(this.extensionUri, "out", "webview"),
      ],
    };

    webviewView.webview.html = getWebviewContent(
      webviewView.webview,
      this.extensionUri,
    );

    // Handle messages from webview
    webviewView.webview.onDidReceiveMessage((msg) => {
      this.handleWebviewMessage(msg);
    });

    // Notify webview of connection status
    if (this.client.connected) {
      this.postMessage({ type: "connected" });
      void this.refreshGoalSnapshot();
      void this.refreshRuntimePolicySnapshot();
    }

    this.client.on("connected", () => {
      this.postMessage({ type: "connected" });
      void this.refreshGoalSnapshot();
      void this.refreshRuntimePolicySnapshot();
    });

    this.client.on("disconnected", () => {
      this.postMessage({ type: "disconnected" });
    });
  }

  postMessage(msg: unknown): void {
    this.view?.webview.postMessage(msg);
  }

  private emitTimeline(
    stage: "received" | "plan" | "tool" | "stream" | "approval" | "result" | "error" | "cancel",
    title: string,
    detail?: string,
    turnId?: string,
  ): void {
    this.postMessage({
      type: "timelineEvent",
      id: crypto.randomUUID(),
      stage,
      title,
      detail,
      turnId,
      timestamp: Date.now(),
    });
  }

  updateAlignment(report: AlignmentReport): void {
    this.postMessage({
      type: "alignmentUpdate",
      score: report.score,
      confidence: report.confidence,
      status: report.status,
    });
  }

  updateGoals(
    goals: Array<{ 
      id: string; 
      description: string; 
      status: string; 
      dependencies?: string[];
      value_to_root?: number;
    }>,
  ): void {
    this.postMessage({
      type: "goalsUpdate",
      goals,
    });
  }

  async refreshGoals(): Promise<void> {
    await this.refreshGoalSnapshot();
  }

  private async handleWebviewMessage(msg: any): Promise<void> {
    const messageType = msg?.type ?? msg?.command;
    switch (messageType) {
      case "chatMessage":
        await this.handleChatMessage(msg.text);
        break;
      case "regenerateLastResponse":
        await this.handleChatMessage(msg.text);
        break;
      case "cancelStreaming":
        if (typeof msg.messageId === "string" && this.activeStreamId === msg.messageId) {
          this.activeStreamId = null;
          this.postMessage({ type: "chatStreamingStopped", id: msg.messageId });
          this.emitTimeline("cancel", "Streaming cancelled", "Stopped by user", msg.messageId);
        }
        break;
      case "clearChatMemory":
        await this.handleClearChatMemory();
        break;

      case "fileApproval":
        this.outputChannel.appendLine(
          `File ${msg.approved ? "approved" : "rejected"}: ${msg.path}`,
        );
        break;

      case "mcpRequest":
        try {
          this.outputChannel.appendLine(
            `Executing MCP request from webview: ${msg.method} (${msg.params?.name})`,
          );
          let result;
          if (msg.method === "tools/call") {
            result = await this.client.callTool(
              msg.params.name,
              msg.params.arguments || {},
            );
          } else {
            // @ts-ignore - for raw requests
            result = await this.client.request(msg.method, msg.params || {});
          }
          this.postMessage({ type: "mcpResponse", result, id: msg.id });
        } catch (err: any) {
          this.outputChannel.appendLine(`MCP request failed: ${err.message}`);
          this.postMessage({
            type: "mcpResponse",
            error: err.message,
            id: msg.id,
          });
        }
        break;

      case "refreshGoals":
        await this.refreshGoalSnapshot();
        break;

      case "refreshRuntimePolicies":
        await this.refreshRuntimePolicySnapshot();
        break;

      case "governanceApprove":
        await this.handleGovernanceApprove(msg.note);
        break;

      case "governanceReject":
        await this.handleGovernanceReject(msg.reason);
        break;

      case "governanceSeed":
        await this.handleGovernanceSeed(Boolean(msg.apply), msg.lockRequired !== false);
        break;

      case "webviewReady":
        if (this.client.connected) {
          this.postMessage({ type: "connected" });
          void this.refreshGoalSnapshot();
          void this.refreshRuntimePolicySnapshot();
        } else {
          this.postMessage({ type: "disconnected" });
        }
        break;

      case "codexLogin":
        vscode.commands.executeCommand(CMD_CODEX_LOGIN);
        break;
    }
  }

  private async handleChatMessage(text: string): Promise<void> {
    if (!this.client.connected) {
      this.postMessage({
        type: "chatResponse",
        id: crypto.randomUUID(),
        content:
          "Sentinel is not connected. Please check that sentinel-cli is installed and accessible.",
      });
      return;
    }

    // Generate a message ID for streaming/updates
    const messageId = crypto.randomUUID();
    this.emitTimeline("received", "Prompt received", text, messageId);

    // ── Command Parsing ──────────────────────────────────────
    if (text.startsWith("/init ")) {
      const description = text.replace("/init ", "").trim();
      this.emitTimeline("plan", "Slash command /init", description, messageId);
      try {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: `🚀 Initializing project: "${description}"...`,
        });

        const result: any = await this.client.callTool("init_project", {
          description,
        });

        this.postMessage({
          type: "chatResponse",
          id: crypto.randomUUID(),
          content: `✅ **Success!**\n${result.text || "Project manifold created."}\n\nSwitch to the **Atomic Forge** tab to see your goals.`,
        });

        // Refresh views
        vscode.commands.executeCommand("sentinel.refreshGoals");
        void this.refreshGoalSnapshot();
        this.emitTimeline("result", "Project initialized", description, messageId);
        return;
      } catch (err: any) {
        this.postMessage({
          type: "chatResponse",
          id: crypto.randomUUID(),
          content: `❌ **Initialization failed:** ${err.message}`,
        });
        this.emitTimeline("error", "Initialization failed", err.message, messageId);
        return;
      }
    }

    if (text.trim() === "/help") {
      this.emitTimeline("plan", "Slash command /help", "Help menu requested", messageId);
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content:
          "Comandi disponibili:\n- `/init <descrizione>`\n- `/clear-memory`\n- `/memory-status`\n- `/memory-search <query>`\n- `/memory-export [path]`\n- `/memory-import <path> [merge=true|false]`",
      });
      return;
    }

    if (text.trim() === "/memory-status") {
      this.emitTimeline("tool", "Memory status", "Querying memory state", messageId);
      const result = (await this.client.callTool("chat_memory_status", {})) as any;
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: `Memory turns: ${result?.turn_count ?? 0}\nRecent:\n${(result?.recent_turns ?? [])
          .map((t: any) => `- ${t.id?.slice(0, 8)} ${t.intent_summary ?? ""}`)
          .join("\n")}`,
      });
      return;
    }

    if (text.startsWith("/memory-search ")) {
      const query = text.replace("/memory-search ", "").trim();
      this.emitTimeline("tool", "Memory search", query, messageId);
      const result = (await this.client.callTool("chat_memory_search", {
        query,
        limit: 8,
      })) as any;
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: `Memory hits (${result?.count ?? 0}) for "${query}":\n${(result?.hits ?? [])
          .map((h: any) => `- ${h.id?.slice(0, 8)}: ${h.intent_summary ?? ""}`)
          .join("\n")}`,
      });
      return;
    }

    if (text.startsWith("/memory-export")) {
      const maybePath = text.replace("/memory-export", "").trim();
      this.emitTimeline("tool", "Memory export", maybePath || "default path", messageId);
      const args: Record<string, unknown> = {};
      if (maybePath) args.path = maybePath;
      const result = (await this.client.callTool("chat_memory_export", args)) as any;
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: result?.ok
          ? `Memory export completato: ${result.path}\nTurns: ${result.turn_count}`
          : `Memory export fallito: ${result?.error ?? "unknown error"}`,
      });
      return;
    }

    if (text.startsWith("/memory-import ")) {
      const payload = text.replace("/memory-import ", "").trim();
      const [pathArg, mergeArg] = payload.split(/\s+/);
      if (!pathArg) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: "Usage: /memory-import <path> [merge=true|false]",
        });
        return;
      }
      const merge = mergeArg ? mergeArg.toLowerCase() !== "merge=false" : true;
      this.emitTimeline("tool", "Memory import", `${pathArg} (merge=${merge})`, messageId);
      const result = (await this.client.callTool("chat_memory_import", {
        path: pathArg,
        merge,
      })) as any;
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: result?.ok
          ? `Memory import completato (${merge ? "merge" : "replace"}): ${result.turn_count} turns`
          : `Memory import fallito: ${result?.error ?? "unknown error"}`,
      });
      return;
    }

    try {
      this.emitTimeline("plan", "Inference planned", "Executing chat tool", messageId);
      // Send a "thinking" state
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: "",
        streaming: true,
      });

      // Use the NEW REAL INFERENCE chat tool
      this.emitTimeline("tool", "MCP tool call", "chat", messageId);
      const result: any = await this.client.callTool("chat", {
        message: text
      });

      let content = "No response from Sentinel.";
      let thoughtChain: string[] | undefined = undefined;
      let explainability: unknown = undefined;
      let streamChunks: string[] = [];

      if (result && typeof result === "object") {
        const structured = result as Record<string, unknown>;
        if (typeof structured.answer === "string") {
          content = structured.answer;
        } else if (result.content && Array.isArray(result.content) && result.content[0]?.text) {
          content = String(result.content[0].text);
        }
        if (Array.isArray(structured.thought_chain)) {
          thoughtChain = structured.thought_chain.map((v) => String(v));
        }
        if (structured.explainability) {
          explainability = structured.explainability;
        }
        if (Array.isArray(structured.stream_chunks)) {
          streamChunks = structured.stream_chunks.map((v) => String(v));
        }
      } else if (typeof result === "string") {
        content = result;
      }

      this.activeStreamId = messageId;
      if (streamChunks.length > 0) {
        this.emitTimeline("stream", "Streaming started", `${streamChunks.length} chunks`, messageId);
        let partial = "";
        for (const chunk of streamChunks) {
          if (this.activeStreamId !== messageId) break;
          partial += chunk;
          this.postMessage({
            type: "chatStreaming",
            id: messageId,
            content: partial,
          });
          await new Promise((resolve) => setTimeout(resolve, 24));
        }
      }
      if (this.activeStreamId === messageId) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content,
          thoughtChain,
          explainability,
          streaming: false,
        });
      }
      this.activeStreamId = null;
      this.emitTimeline("result", "Turn completed", "Response delivered", messageId);

      // After chat, refresh goals in background to keep UI synced
      void this.refreshGoalSnapshot();
      void this.refreshRuntimePolicySnapshot();
      
    } catch (err: any) {
      this.outputChannel.appendLine(`Chat tool error: ${err.message}`);
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: `Error: ${err.message}. Ensure LLM API keys are configured.`,
        streaming: false,
      });
      this.emitTimeline("error", "Chat tool error", err.message, messageId);
    }
  }

  private async refreshGoalSnapshot(): Promise<void> {
    if (!this.client.connected) {
      this.updateGoals([]);
      return;
    }

    try {
      const graph: any = await this.client.callTool("get_goal_graph", {});
      const nodes = Array.isArray(graph?.nodes) ? graph.nodes : [];
      const edges = Array.isArray(graph?.edges) ? graph.edges : [];

      const goals = nodes
        .filter((node: any) => node?.id && node.id !== "root")
        .map((node: any) => {
          // Find dependencies where this node is the target (source -> target)
          const nodeDependencies = edges
            .filter((e: any) => e.target === node.id)
            .map((e: any) => e.source);

          return {
            id: String(node.id),
            description: String(node.data?.label ?? ""),
            status: String(node.data?.status ?? "Unknown"),
            dependencies: nodeDependencies,
            value_to_root: node.data?.value ?? 0,
          };
        })
        .filter((goal: any) => goal.description.trim().length > 0);

      this.updateGoals(goals);
    } catch (err: any) {
      this.outputChannel.appendLine(`Failed to refresh goals: ${err.message}`);
    }
  }

  private async refreshRuntimePolicySnapshot(): Promise<void> {
    if (!this.client.connected) {
      return;
    }

    try {
      const reliability = (await this.client.callTool("get_reliability", {})) as any;
      if (reliability && !reliability.error) {
        this.postMessage({
          type: "reliabilityUpdate",
          reliability: reliability.reliability,
          reliability_thresholds: reliability.reliability_thresholds,
          reliability_slo: reliability.reliability_slo,
        });
      }
    } catch (err: any) {
      this.outputChannel.appendLine(`Failed to refresh reliability: ${err.message}`);
    }

    try {
      const governance = (await this.client.callTool("governance_status", {})) as any;
      if (governance && !governance.error) {
        this.postMessage({
          type: "governanceUpdate",
          governance,
        });
      }
    } catch (err: any) {
      this.outputChannel.appendLine(`Failed to refresh governance: ${err.message}`);
    }
  }

  private async handleGovernanceApprove(note?: string): Promise<void> {
    if (!this.client.connected) return;
    try {
      this.emitTimeline("approval", "Governance approve requested", note ?? "", undefined);
      const result = (await this.client.callTool("governance_approve", {
        note: typeof note === "string" ? note : "",
      })) as any;
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_approve",
        ok: true,
        message: result?.message ?? "Governance proposal approved.",
      });
      this.emitTimeline("result", "Governance proposal approved", result?.proposal_id, undefined);
      await this.refreshRuntimePolicySnapshot();
    } catch (err: any) {
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_approve",
        ok: false,
        message: err.message,
      });
      this.emitTimeline("error", "Governance approve failed", err.message, undefined);
    }
  }

  private async handleGovernanceReject(reason?: string): Promise<void> {
    if (!this.client.connected) return;
    try {
      this.emitTimeline("approval", "Governance reject requested", reason ?? "", undefined);
      const result = (await this.client.callTool("governance_reject", {
        reason: typeof reason === "string" ? reason : "Rejected from VSCode UI",
      })) as any;
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_reject",
        ok: true,
        message: result?.message ?? "Governance proposal rejected.",
      });
      this.emitTimeline("result", "Governance proposal rejected", result?.proposal_id, undefined);
      await this.refreshRuntimePolicySnapshot();
    } catch (err: any) {
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_reject",
        ok: false,
        message: err.message,
      });
      this.emitTimeline("error", "Governance reject failed", err.message, undefined);
    }
  }

  private async handleGovernanceSeed(
    apply: boolean,
    lockRequired: boolean,
  ): Promise<void> {
    if (!this.client.connected) return;
    try {
      this.emitTimeline(
        "tool",
        "Governance seed requested",
        `apply=${apply} lock_required=${lockRequired}`,
        undefined,
      );
      const result = (await this.client.callTool("governance_seed", {
        apply,
        lock_required: lockRequired,
      })) as any;

      const message = apply
        ? result?.message ?? "Governance baseline updated."
        : `Preview generated: deps+${result?.diff?.dependencies?.add?.length ?? 0} deps-${result?.diff?.dependencies?.remove?.length ?? 0}`;
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_seed",
        ok: true,
        message,
      });
      this.emitTimeline("result", "Governance seed completed", message, undefined);

      if (apply) {
        await this.refreshRuntimePolicySnapshot();
      } else {
        this.postMessage({
          type: "governanceUpdate",
          governance: {
            ...(await this.client.callTool("governance_status", {})),
            seed_preview: result?.diff ?? null,
            observed: result?.observed ?? null,
          },
        });
      }
    } catch (err: any) {
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_seed",
        ok: false,
        message: err.message,
      });
      this.emitTimeline("error", "Governance seed failed", err.message, undefined);
    }
  }

  private async handleClearChatMemory(): Promise<void> {
    if (!this.client.connected) return;
    try {
      this.emitTimeline("tool", "Memory clear", "chat_memory_clear", undefined);
      const result = (await this.client.callTool("chat_memory_clear", {})) as any;
      this.postMessage({
        type: "policyActionResult",
        kind: "chat_memory_clear",
        ok: result?.ok !== false,
        message: result?.message ?? "Chat memory cleared.",
      });
    } catch (err: any) {
      this.postMessage({
        type: "policyActionResult",
        kind: "chat_memory_clear",
        ok: false,
        message: err.message,
      });
    }
  }
}
