import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import { spawn } from "child_process";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";
import { MCPClient } from "./mcp/client";
import { SentinelStatusBar } from "./providers/statusBar";
import { SentinelCodeLensProvider } from "./providers/codeLensProvider";
import { SentinelChatViewProvider } from "./webview-providers/SentinelChatViewProvider";
import { sentinelService } from "./services/SentinelService";
import {
  CONFIG_SENTINEL_PATH,
  CONFIG_DEFAULT_PATH,
  CMD_OPEN_CHAT,
  CMD_REFRESH_GOALS,
  CMD_VALIDATE_ACTION,
  CMD_SHOW_ALIGNMENT,
  CMD_CODEX_LOGIN,
  POLL_INTERVAL_MS,
} from "./shared/constants";

let lspClient: LanguageClient | undefined;
let mcpClient: MCPClient | undefined;
let pollTimer: ReturnType<typeof setInterval> | undefined;

/**
 * Resolves the absolute path to the sentinel binary.
 * Tries the configured path, then common locations like ~/.local/bin and ~/.cargo/bin.
 */
function resolveSentinelPath(
  configuredPath: string,
  outputChannel: vscode.OutputChannel,
): string {
  if (path.isAbsolute(configuredPath)) {
    return configuredPath;
  }

  const home = process.env.HOME || process.env.USERPROFILE;
  if (home) {
    const locations = [
      path.join(home, ".local", "bin", configuredPath),
      path.join(home, ".cargo", "bin", configuredPath),
      path.join("/usr", "local", "bin", configuredPath),
    ];

    for (const loc of locations) {
      if (fs.existsSync(loc)) {
        outputChannel.appendLine(`Resolved sentinel path: ${loc}`);
        return loc;
      }
    }
  }

  outputChannel.appendLine(`Using configured path directly: ${configuredPath}`);
  return configuredPath;
}

export function activate(context: vscode.ExtensionContext) {
  const outputChannel = vscode.window.createOutputChannel("Sentinel");
  context.subscriptions.push(outputChannel);

  let rawPath =
    vscode.workspace
      .getConfiguration("sentinel")
      .get<string>(CONFIG_SENTINEL_PATH) || CONFIG_DEFAULT_PATH;
  rawPath = rawPath.replace(/^["'](.+)["']$/, "$1");

  const sentinelPath = resolveSentinelPath(rawPath, outputChannel);

  const workspaceRoot = vscode.workspace.workspaceFolders
    ? vscode.workspace.workspaceFolders[0].uri.fsPath
    : ".";

  outputChannel.appendLine(`Activating Sentinel extension...`);
  outputChannel.appendLine(`Workspace: ${workspaceRoot}`);
  outputChannel.appendLine(`Binary path: ${sentinelPath}`);

  // Initialize SDK Service (Dogfooding)
  sentinelService.initialize(workspaceRoot).then(success => {
    if (success) outputChannel.appendLine("✅ SDK Service Initialized");
    else outputChannel.appendLine("❌ SDK Service Failed");
  });

  const buildMcpEnv = () => {
    const useOpenAIAuth = context.globalState.get<boolean>(
      "sentinel.useOpenAIAuth",
      false,
    );
    if (!useOpenAIAuth) return {};
    return {
      SENTINEL_OPENAI_AUTH: "true",
      SENTINEL_LLM_PROVIDER: "openai_auth",
    } as NodeJS.ProcessEnv;
  };

  // ── 1. MCP Client ────────────────────────────────────────
  mcpClient = new MCPClient(
    sentinelPath,
    workspaceRoot,
    outputChannel,
    buildMcpEnv(),
  );

  // Start MCP connection
  mcpClient.start().catch((err) => {
    outputChannel.appendLine(`MCP initial connection error: ${err.message}`);
  });

  // ── 2. LSP Client ────────────────────────────────────────
  const serverOptions: ServerOptions = {
    command: sentinelPath,
    args: ["lsp"],
    options: { cwd: workspaceRoot },
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "rust" },
      { scheme: "file", language: "typescript" },
      { scheme: "file", language: "javascript" },
      { scheme: "file", language: "python" },
    ],
  };

  lspClient = new LanguageClient(
    "sentinelLSP",
    "Sentinel LSP",
    serverOptions,
    clientOptions,
  );
  lspClient.start();

  // ── 3. Status Bar ────────────────────────────────────────
  const statusBar = new SentinelStatusBar(mcpClient);
  context.subscriptions.push({ dispose: () => statusBar.dispose() });

  // ── 4. CodeLens ──────────────────────────────────────────
  const codeLensProvider = new SentinelCodeLensProvider(mcpClient);
  context.subscriptions.push(
    vscode.languages.registerCodeLensProvider(
      [
        { language: "rust" },
        { language: "typescript" },
        { language: "javascript" },
        { language: "python" },
      ],
      codeLensProvider,
    ),
  );

  // ── 5. Chat Webview ──────────────────────────────────────
  const chatProvider = new SentinelChatViewProvider(
    context.extensionUri,
    mcpClient,
    outputChannel,
  );
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      SentinelChatViewProvider.viewId,
      chatProvider,
      { webviewOptions: { retainContextWhenHidden: true } },
    ),
  );

  // ── 6. Commands ──────────────────────────────────────────
  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_OPEN_CHAT, () => {
      vscode.commands.executeCommand("sentinel-chat.focus");
    }),

    vscode.commands.registerCommand(CMD_CODEX_LOGIN, async () => {
      try {
        await vscode.window.withProgress(
          {
            location: vscode.ProgressLocation.Notification,
            title: "Sentinel: Sign in with ChatGPT",
            cancellable: true,
          },
          async (_progress, token) => {
            const login = () =>
              new Promise<void>((resolve, reject) => {
                outputChannel.appendLine("Starting Codex login...");
                const proc = spawn("codex", ["login"], {
                  cwd: workspaceRoot,
                  shell: true,
                  env: process.env,
                });

                let stderr = "";
                proc.stderr?.on("data", (chunk: Buffer) => {
                  const text = chunk.toString("utf-8");
                  stderr += text;
                  outputChannel.appendLine(`[codex] ${text.trim()}`);
                });

                proc.on("error", (err) => reject(err));
                proc.on("close", (code) => {
                  if (code === 0) {
                    resolve();
                  } else {
                    reject(
                      new Error(
                        stderr.trim() ||
                          `Codex login failed (exit ${code ?? "unknown"})`,
                      ),
                    );
                  }
                });

                token.onCancellationRequested(() => {
                  outputChannel.appendLine("Codex login cancelled.");
                  proc.kill();
                  reject(new Error("Codex login cancelled."));
                });
              });

            await login();
            await context.globalState.update("sentinel.useOpenAIAuth", true);
            const env = buildMcpEnv();
            mcpClient?.setEnvOverrides(env);
            mcpClient?.disconnect();
            await mcpClient?.start();
            vscode.window.showInformationMessage(
              "ChatGPT sign-in complete. Sentinel now uses OpenAI OAuth via Codex.",
            );
          },
        );
      } catch (err: any) {
        vscode.window.showErrorMessage(`Codex login failed: ${err.message}`);
      }
    }),

    vscode.commands.registerCommand(CMD_REFRESH_GOALS, async () => {
      await chatProvider.refreshGoals();
    }),

    vscode.commands.registerCommand(CMD_VALIDATE_ACTION, async () => {
      if (!mcpClient?.connected) {
        vscode.window.showWarningMessage("Sentinel is not connected.");
        return;
      }

      const description = await vscode.window.showInputBox({
        prompt: "Describe the action to validate",
        placeHolder: "e.g., Implement JWT authentication",
      });

      if (!description) return;

      try {
        const result = await mcpClient.validateAction(
          "user_action",
          description,
        );
        const msg = `Alignment: ${result.alignment_score.toFixed(1)}% | Risk: ${result.risk_level} | ${result.approved ? "APPROVED" : "REJECTED"}: ${result.rationale}`;
        if (result.approved) {
          vscode.window.showInformationMessage(msg);
        } else {
          vscode.window.showWarningMessage(msg);
        }
      } catch (err: any) {
        vscode.window.showErrorMessage(`Validation failed: ${err.message}`);
      }
    }),

    vscode.commands.registerCommand(CMD_SHOW_ALIGNMENT, async () => {
      if (!mcpClient?.connected) {
        vscode.window.showWarningMessage(
          "Sentinel is not connected. Ensure sentinel-cli is installed and accessible.",
        );
        return;
      }

      try {
        const report = await mcpClient.getAlignment();
        const lines = [
          `Score: ${report.score.toFixed(1)}%`,
          `Confidence: ${(report.confidence * 100).toFixed(0)}%`,
          `Status: ${report.status}`,
        ];
        if (report.violations.length > 0) {
          lines.push("", "Violations:");
          for (const v of report.violations) {
            lines.push(`  - ${v.description} (severity: ${v.severity})`);
          }
        }
        vscode.window.showInformationMessage(lines.join("\n"), { modal: true });
      } catch (err: any) {
        vscode.window.showErrorMessage(
          `Failed to get alignment: ${err.message}`,
        );
      }
    }),
  );

  // ── 7. Polling ───────────────────────────────────────────
  pollTimer = setInterval(async () => {
    if (!mcpClient?.connected) return;

    try {
      const report = await mcpClient.getAlignment();

      // Update all consumers
      statusBar.update(report);
      codeLensProvider.updateReport(report);
      chatProvider.updateAlignment(report);
    } catch {
      // Silently fail on poll; will retry next interval
    }
  }, POLL_INTERVAL_MS);

  // ── 8. Connection status updates ─────────────────────────
  mcpClient.on("connected", () => {
    outputChannel.appendLine("Sentinel MCP connected");
    // Trigger initial data load
    vscode.commands.executeCommand(CMD_REFRESH_GOALS);
  });

  mcpClient.on("disconnected", () => {
    statusBar.showDisconnected();
    outputChannel.appendLine("Sentinel MCP disconnected");
  });

  outputChannel.appendLine(
    `Sentinel extension activated. Path: ${sentinelPath}`,
  );
}

export function deactivate(): Thenable<void> | undefined {
  if (pollTimer) {
    clearInterval(pollTimer);
  }

  mcpClient?.stop();

  if (!lspClient) {
    return undefined;
  }
  return lspClient.stop();
}
