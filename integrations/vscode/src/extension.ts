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
import {
  AugmentRuntimeSettings,
  SentinelChatViewProvider,
} from "./webview-providers/SentinelChatViewProvider";
import { sentinelService } from "./services/SentinelService";
import { LivePreviewProvider, devServerDetector, DevServer } from "./services";
import {
  CONFIG_SENTINEL_PATH,
  CONFIG_DEFAULT_PATH,
  CMD_OPEN_CHAT,
  CMD_REFRESH_GOALS,
  CMD_VALIDATE_ACTION,
  CMD_SHOW_ALIGNMENT,
  CMD_CODEX_LOGIN,
  CMD_BLUEPRINT_LIST,
  CMD_BLUEPRINT_SHOW,
  CMD_BLUEPRINT_APPLY,
  CMD_BLUEPRINT_QUICKSTART,
  CMD_PREVIEW_TOGGLE,
  CMD_PREVIEW_REFRESH,
  CMD_PREVIEW_VIEWPORT_DESKTOP,
  CMD_PREVIEW_VIEWPORT_TABLET,
  CMD_PREVIEW_VIEWPORT_MOBILE,
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
  workspaceRoot: string,
  outputChannel: vscode.OutputChannel,
): string {
  if (path.isAbsolute(configuredPath)) {
    return configuredPath;
  }

  const workspaceCandidates = [
    path.join(workspaceRoot, "target", "release", "sentinel-cli"),
    path.join(workspaceRoot, "target", "release", "sentinel"),
    path.join(workspaceRoot, "target", "debug", "sentinel-cli"),
    path.join(workspaceRoot, "target", "debug", "sentinel"),
  ];
  for (const candidate of workspaceCandidates) {
    if (fs.existsSync(candidate)) {
      outputChannel.appendLine(`Resolved sentinel path from workspace build: ${candidate}`);
      return candidate;
    }
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

  const resolveWorkspaceRoot = (): string => {
    if (vscode.workspace.workspaceFolders?.length) {
      return vscode.workspace.workspaceFolders[0].uri.fsPath;
    }

    // Fallback: walk up from extension path to find sentinel.json
    let current = context.extensionPath;
    while (current && current !== path.dirname(current)) {
      const candidate = path.join(current, "sentinel.json");
      if (fs.existsSync(candidate)) {
        outputChannel.appendLine(
          `Workspace not set. Using sentinel.json at: ${candidate}`,
        );
        return current;
      }
      current = path.dirname(current);
    }

    return ".";
  };

  const workspaceRoot = resolveWorkspaceRoot();

  let rawPath =
    vscode.workspace
      .getConfiguration("sentinel")
      .get<string>(CONFIG_SENTINEL_PATH) || CONFIG_DEFAULT_PATH;
  rawPath = rawPath.replace(/^["'](.+)["']$/, "$1");

  const sentinelPath = resolveSentinelPath(rawPath, workspaceRoot, outputChannel);

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
    const augment = context.globalState.get<AugmentRuntimeSettings>(
      "sentinel.augmentSettings",
      { enabled: false, mode: "disabled", enforceByo: true },
    );
    const env: NodeJS.ProcessEnv = {};
    if (useOpenAIAuth) {
      env.SENTINEL_OPENAI_AUTH = "true";
      env.SENTINEL_LLM_PROVIDER = "openai_auth";
    }
    env.SENTINEL_AUGMENT_ENABLED = augment.enabled ? "true" : "false";
    env.SENTINEL_AUGMENT_MODE = augment.mode;
    env.SENTINEL_AUGMENT_ENFORCE_BYO = augment.enforceByo ? "true" : "false";
    env.SENTINEL_CONTEXT_PROVIDER_PRIORITY =
      "qdrant_mcp,filesystem_mcp,git_mcp,memory_mcp,augment_mcp";
    return env;
  };

  const applyMcpEnvAndReconnect = async () => {
    const env = buildMcpEnv();
    mcpClient?.setEnvOverrides(env);
    mcpClient?.disconnect();
    await mcpClient?.start();
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
    context,
    async () => {
      await applyMcpEnvAndReconnect();
    },
  );
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      SentinelChatViewProvider.viewId,
      chatProvider,
      { webviewOptions: { retainContextWhenHidden: true } },
    ),
  );

  // ── 6. Live Preview ──────────────────────────────────────
  const livePreviewProvider = new LivePreviewProvider(context);
  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      LivePreviewProvider.viewType,
      livePreviewProvider,
      { webviewOptions: { retainContextWhenHidden: true } }
    )
  );

  // ── 7. Commands ──────────────────────────────────────────
  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_OPEN_CHAT, () => {
      void vscode.commands.executeCommand("sentinel-chat.focus").then(
        () => undefined,
        () => vscode.commands.executeCommand("workbench.view.extension.sentinel-explorer"),
      );
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
            await applyMcpEnvAndReconnect();
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
      outputChannel.appendLine("Refreshing goals...");
      await chatProvider.refreshGoals();
      vscode.window.showInformationMessage("Goals refreshed");
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

    // Blueprint commands
    vscode.commands.registerCommand(CMD_BLUEPRINT_LIST, async () => {
      outputChannel.appendLine("Listing blueprints...");
      try {
        const result = spawn(sentinelPath, ["blueprint", "list", "--json"], {
          cwd: workspaceRoot,
          shell: true,
        });

        let stdout = "";
        let stderr = "";

        result.stdout?.on("data", (chunk: Buffer) => {
          stdout += chunk.toString();
        });

        result.stderr?.on("data", (chunk: Buffer) => {
          stderr += chunk.toString();
        });

        result.on("close", async (code) => {
          if (code === 0 && stdout) {
            try {
              const blueprints = JSON.parse(stdout);
              const items = blueprints.map((bp: any) => ({
                label: bp.name,
                description: bp.description,
                detail: `${bp.category} | ${bp.difficulty} | ${bp.estimated_time}`,
              }));

              const selected = await vscode.window.showQuickPick(items, {
                placeHolder: "Select a blueprint to view details",
              });

              if (selected) {
                await vscode.commands.executeCommand(CMD_BLUEPRINT_SHOW, selected.label);
              }
            } catch (e) {
              outputChannel.appendLine(`Failed to parse blueprint list: ${e}`);
            }
          } else {
            vscode.window.showErrorMessage(`Failed to list blueprints: ${stderr}`);
          }
        });
      } catch (err: any) {
        vscode.window.showErrorMessage(`Failed to list blueprints: ${err.message}`);
      }
    }),

    vscode.commands.registerCommand(CMD_BLUEPRINT_SHOW, async (name?: string) => {
      const blueprintName = name ?? await vscode.window.showInputBox({
        prompt: "Enter blueprint name",
        placeHolder: "e.g., web-app-auth-crud-board",
      });

      if (!blueprintName) return;

      outputChannel.appendLine(`Showing blueprint: ${blueprintName}`);
      try {
        const result = spawn(sentinelPath, ["blueprint", "show", blueprintName], {
          cwd: workspaceRoot,
          shell: true,
        });

        let stdout = "";
        let stderr = "";

        result.stdout?.on("data", (chunk: Buffer) => {
          stdout += chunk.toString();
        });

        result.stderr?.on("data", (chunk: Buffer) => {
          stderr += chunk.toString();
        });

        result.on("close", (code) => {
          if (code === 0) {
            // Create a new untitled document with the blueprint details
            vscode.workspace.openTextDocument({
              content: stdout,
              language: "markdown",
            }).then(doc => {
                vscode.window.showTextDocument(doc);
              });
          } else {
            vscode.window.showErrorMessage(`Failed to show blueprint: ${stderr}`);
          }
        });
      } catch (err: any) {
        vscode.window.showErrorMessage(`Failed to show blueprint: ${err.message}`);
      }
    }),

    vscode.commands.registerCommand(CMD_BLUEPRINT_APPLY, async (name?: string) => {
      const blueprintName = name ?? await vscode.window.showInputBox({
        prompt: "Enter blueprint name to apply",
        placeHolder: "e.g., web-app-auth-crud-board",
      });

      if (!blueprintName) return;

      const confirm = await vscode.window.showWarningMessage(
        `Apply blueprint '${blueprintName}' to current workspace?`,
        { modal: true },
        "Apply",
      );

      if (confirm !== "Apply") return;

      outputChannel.appendLine(`Applying blueprint: ${blueprintName}`);
      try {
        await vscode.window.withProgress(
          {
            location: vscode.ProgressLocation.Notification,
            title: `Applying blueprint '${blueprintName}'...`,
            cancellable: false,
          },
          async () => {
            const result = spawn(sentinelPath, ["blueprint", "apply", blueprintName], {
              cwd: workspaceRoot,
              shell: true,
            });

            let stdout = "";
            let stderr = "";

            result.stdout?.on("data", (chunk: Buffer) => {
              stdout += chunk.toString();
            });

            result.stderr?.on("data", (chunk: Buffer) => {
              stderr += chunk.toString();
            });

            return new Promise<void>((resolve, reject) => {
              result.on("close", (code) => {
                if (code === 0) {
                  vscode.window.showInformationMessage(`Blueprint applied successfully!\n${stdout}`);
                  resolve();
                } else {
                  vscode.window.showErrorMessage(`Failed to apply blueprint: ${stderr}`);
                  reject(new Error(stderr));
                }
              });
            });
          },
        );
      } catch (err: any) {
        vscode.window.showErrorMessage(`Failed to apply blueprint: ${err.message}`);
      }
    }),

    vscode.commands.registerCommand(CMD_BLUEPRINT_QUICKSTART, async () => {
      // Show quickstart dialog with blueprint options
      const options = [
        { label: "$(browser) Web App with Auth", value: "web-app-auth-crud-board", description: "Full-stack web app with authentication, CRUD, and kanban board" },
        { label: "$(server) Backend API", value: "backend-api-auth-billing", description: "REST API with authentication and Stripe billing" },
        { label: "$(beaker) CI/CD Pipeline", value: "integration-ci-quality-gates", description: "CI/CD integration with automated quality gates" },
        { label: "$(git-branch) Migration Blueprint", value: "migration-monolith-to-modular", description: "Monolith to microservices migration guide" },
      ];

      const selected = await vscode.window.showQuickPick(options, {
        placeHolder: "Choose a blueprint to quickstart your project",
      });

      if (selected) {
        await vscode.commands.executeCommand(CMD_BLUEPRINT_APPLY, selected.value);
      }
    }),

    // Live Preview Commands
    vscode.commands.registerCommand(CMD_PREVIEW_TOGGLE, async () => {
      const state = livePreviewProvider.getState();
      if (state.server) {
        livePreviewProvider.stopPreview();
        vscode.window.showInformationMessage("Live Preview stopped");
      } else {
        const result = await devServerDetector.detectServers();
        if (result.servers.length > 0) {
          await livePreviewProvider.startPreview(result.servers[0]);
          void vscode.commands.executeCommand("sentinel-live-preview.focus");
          vscode.window.showInformationMessage(`Live Preview started: ${result.servers[0].type} on port ${result.servers[0].port}`);
        } else {
          vscode.window.showWarningMessage(
            "No development server detected. Start your dev server (e.g., npm run dev) and try again."
          );
        }
      }
    }),

    vscode.commands.registerCommand(CMD_PREVIEW_REFRESH, () => {
      livePreviewProvider.refresh();
      vscode.window.showInformationMessage("Live Preview refreshed");
    }),

    vscode.commands.registerCommand(CMD_PREVIEW_VIEWPORT_DESKTOP, () => {
      void livePreviewProvider.changeViewport('desktop');
    }),

    vscode.commands.registerCommand(CMD_PREVIEW_VIEWPORT_TABLET, () => {
      void livePreviewProvider.changeViewport('tablet');
    }),

    vscode.commands.registerCommand(CMD_PREVIEW_VIEWPORT_MOBILE, () => {
      void livePreviewProvider.changeViewport('mobile');
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
