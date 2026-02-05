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
    }

    this.client.on("connected", () => {
      this.postMessage({ type: "connected" });
      void this.refreshGoalSnapshot();
    });

    this.client.on("disconnected", () => {
      this.postMessage({ type: "disconnected" });
    });
  }

  postMessage(msg: unknown): void {
    this.view?.webview.postMessage(msg);
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

      case "webviewReady":
        if (this.client.connected) {
          this.postMessage({ type: "connected" });
          void this.refreshGoalSnapshot();
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

    // ── Command Parsing ──────────────────────────────────────
    if (text.startsWith("/init ")) {
      const description = text.replace("/init ", "").trim();
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
        return;
      } catch (err: any) {
        this.postMessage({
          type: "chatResponse",
          id: crypto.randomUUID(),
          content: `❌ **Initialization failed:** ${err.message}`,
        });
        return;
      }
    }

    try {
      // Send a "thinking" state
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: "",
        streaming: true,
      });

      // Use the NEW REAL INFERENCE chat tool
      const result: any = await this.client.callTool("chat", {
        message: text
      });

      let content = "No response from Sentinel.";
      if (result && result.content && result.content[0] && result.content[0].text) {
        content = result.content[0].text;
      } else if (typeof result === 'string') {
        content = result;
      }

      // Update with the final response
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: content,
        streaming: false,
      });

      // After chat, refresh goals in background to keep UI synced
      void this.refreshGoalSnapshot();
      
    } catch (err: any) {
      this.outputChannel.appendLine(`Chat tool error: ${err.message}`);
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: `Error: ${err.message}. Ensure LLM API keys are configured.`,
        streaming: false,
      });
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
}
