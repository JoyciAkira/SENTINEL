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
import ProviderConfigService from "./services/ProviderConfigService";
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
  CMD_CONFIGURE_PROVIDERS,
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

  // Register view/title and high-visibility commands first so they exist before workbench resolves menus
  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_OPEN_CHAT, () => {
      void vscode.commands.executeCommand("sentinel-chat.focus").then(
        () => undefined,
        () => vscode.commands.executeCommand("workbench.view.explorer"),
      );
    }),
  );
  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_REFRESH_GOALS, async () => {
      if (!mcpClient?.connected) {
        outputChannel.appendLine("⚠️ MCP not connected, cannot refresh goals");
        return;
      }
      try {
        await mcpClient.getGoals();
        await mcpClient.getAlignment();
        outputChannel.appendLine("✅ Goals refreshed");
      } catch (err) {
        outputChannel.appendLine(`❌ Failed to refresh goals: ${err}`);
      }
    }),
  );

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

  const buildMcpEnv = async () => {
    const useOpenAIAuth = context.globalState.get<boolean>(
      "sentinel.useOpenAIAuth",
      false,
    );
    const providerService = ProviderConfigService.getInstance(context);
    const openaiAuthEnabled = await providerService.isProviderEnabled(
      "openai_auth",
      useOpenAIAuth,
    );
    const providers = await providerService.getAllProviders();
    const enabledConfigured = providers.filter((p) => p.isConfigured && p.isEnabled);
    const augment = context.globalState.get<AugmentRuntimeSettings>(
      "sentinel.augmentSettings",
      { enabled: false, mode: "disabled", enforceByo: true },
    );
    const env: NodeJS.ProcessEnv = {};
    if (openaiAuthEnabled) {
      env.SENTINEL_OPENAI_AUTH = "true";
      env.SENTINEL_LLM_PROVIDER = "openai_auth";
      env.SENTINEL_DISABLE_OPENAI_AUTH = "false";
    } else {
      env.SENTINEL_OPENAI_AUTH = "false";
      env.SENTINEL_DISABLE_OPENAI_AUTH = "true";
    }
    for (const provider of enabledConfigured) {
      if (provider.id === "openai_auth") continue;
      const key = await providerService.getApiKey(provider.id);
      if (!key) continue;
      switch (provider.id) {
        case "openrouter":
          env.OPENROUTER_API_KEY = key;
          break;
        case "openai":
          env.OPENAI_API_KEY = key;
          break;
        case "anthropic":
          env.ANTHROPIC_API_KEY = key;
          break;
        case "google":
          env.GEMINI_API_KEY = key;
          break;
        case "groq":
          env.GROQ_API_KEY = key;
          break;
        case "ollama":
          env.SENTINEL_LLM_BASE_URL = key;
          env.SENTINEL_LLM_MODEL = provider.defaultModel;
          break;
      }
    }
    const firstEnabled = enabledConfigured.find((p) => p.id !== "openai_auth");
    if (!openaiAuthEnabled && firstEnabled) {
      const map: Record<string, string> = {
        openrouter: "openrouter",
        openai: "openai",
        anthropic: "anthropic",
        google: "gemini",
        groq: "openai_compatible",
        ollama: "openai_compatible",
      };
      const mapped = map[firstEnabled.id];
      if (mapped) env.SENTINEL_LLM_PROVIDER = mapped;
    }
    env.SENTINEL_AUGMENT_ENABLED = augment.enabled ? "true" : "false";
    env.SENTINEL_AUGMENT_MODE = augment.mode;
    env.SENTINEL_AUGMENT_ENFORCE_BYO = augment.enforceByo ? "true" : "false";
    env.SENTINEL_CONTEXT_PROVIDER_PRIORITY =
      "qdrant_mcp,filesystem_mcp,git_mcp,memory_mcp,augment_mcp";
    return env;
  };

  const applyMcpEnvAndReconnect = async () => {
    const env = await buildMcpEnv();
    mcpClient?.setEnvOverrides(env);
    mcpClient?.disconnect();
    await mcpClient?.start();
  };

  // ── 1. MCP Client ────────────────────────────────────────
  mcpClient = new MCPClient(
    sentinelPath,
    workspaceRoot,
    outputChannel,
    {},
  );

  // Start MCP connection
  void buildMcpEnv().then((env) => {
    mcpClient?.setEnvOverrides(env);
    return mcpClient?.start();
  }).catch((err) => {
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

  // Bridge: forward detected dev server URL to the chat webview so that
  // the LivePreviewPanel component receives it via postMessage({ type: "livePreviewUrl" }).
  livePreviewProvider.setOnServerDetected((url: string) => {
    outputChannel.appendLine(`Live preview server detected: ${url}`);
    chatProvider.postMessage({ type: "livePreviewUrl", url });
  });

  context.subscriptions.push(
    vscode.window.registerWebviewViewProvider(
      LivePreviewProvider.viewType,
      livePreviewProvider,
      { webviewOptions: { retainContextWhenHidden: true } }
    )
  );

  // ── 7. Provider Configuration ────────────────────────────
  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_CONFIGURE_PROVIDERS, async () => {
      // Open the chat panel with provider configuration.
      // Fallback to the extension view container if focus command is not yet available.
      try {
        await vscode.commands.executeCommand("sentinel-chat.focus");
      } catch {
        await vscode.commands.executeCommand("workbench.view.extension.sentinel-explorer");
        await vscode.commands.executeCommand("sentinel-chat.focus");
      }

      // Send message to webview to show provider config.
      // If view is still resolving, provider queues and flushes on resolve.
      chatProvider.postMessage({
        type: "showProviderConfig",
      });

      outputChannel.appendLine("Opening provider configuration");
    }),
  );

  // ── 8. Remaining Commands (openChat + refreshGoals registered early above) ───
  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_VALIDATE_ACTION, async () => {
      outputChannel.appendLine("Validating actions...");
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_SHOW_ALIGNMENT, async () => {
      if (!mcpClient?.connected) {
        vscode.window.showWarningMessage("Sentinel not connected");
        return;
      }
      try {
        const alignment = await mcpClient.getAlignment();
        vscode.window.showInformationMessage(
          `Alignment: ${alignment.score}% - ${alignment.status}`
        );
      } catch (err) {
        outputChannel.appendLine(`Error getting alignment: ${err}`);
      }
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_BLUEPRINT_QUICKSTART, async () => {
      await vscode.commands.executeCommand('sentinel-chat.focus');
      chatProvider.postMessage({ type: 'showBlueprints' });
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_PREVIEW_TOGGLE, async () => {
      await vscode.commands.executeCommand('sentinel-live-preview.focus');
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand(CMD_PREVIEW_REFRESH, async () => {
      livePreviewProvider.refresh();
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sentinel.preview.viewportDesktop", () => {
      livePreviewProvider.changeViewport('desktop');
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sentinel.preview.viewportTablet", () => {
      livePreviewProvider.changeViewport('tablet');
    }),
  );

  context.subscriptions.push(
    vscode.commands.registerCommand("sentinel.preview.viewportMobile", () => {
      livePreviewProvider.changeViewport('mobile');
    }),
  );

  // ── 9. Connection status updates ─────────────────────────
  mcpClient.on("connected", () => {
    outputChannel.appendLine("Sentinel MCP connected");
    setTimeout(() => {
      // executeCommand returns Thenable, not Promise — wrap to get .catch()
      void Promise.resolve(vscode.commands.executeCommand(CMD_REFRESH_GOALS)).catch((err: Error) => {
        if (err?.message?.includes("not found")) return;
        outputChannel.appendLine(`Failed to refresh goals on connect: ${err.message}`);
      });
    }, 100);
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
