import * as vscode from "vscode";
import * as path from "path";
import { promises as fs } from "fs";
import { MCPClient } from "../mcp/client";
import { getWebviewContent } from "./getWebviewContent";
import type { AlignmentReport } from "../shared/types";
import { CMD_CODEX_LOGIN } from "../shared/constants";

type AugmentRuntimeMode = "disabled" | "internal_only" | "byo_customer";

export interface AugmentRuntimeSettings {
  enabled: boolean;
  mode: AugmentRuntimeMode;
  enforceByo: boolean;
}

const DEFAULT_AUGMENT_SETTINGS: AugmentRuntimeSettings = {
  enabled: false,
  mode: "disabled",
  enforceByo: true,
};

interface ChatSectionPayload {
  id: string;
  title: string;
  content: string;
  language?: string;
  pathHint?: string;
}

interface PendingWritePlanEntry extends ChatSectionPayload {
  path: string;
  applied: boolean;
}

interface ParsedChatPlan {
  normalizedContent: string;
  sections: ChatSectionPayload[];
  fileOperations: Array<{
    path: string;
    type: "create" | "edit" | "delete";
    linesAdded: number;
    linesRemoved: number;
    diff: string;
  }>;
  writeEntries: PendingWritePlanEntry[];
}

type AppSpecFieldType = "string" | "number" | "boolean" | "date" | "enum";

interface AppSpecPayload {
  version: "1.0";
  app: {
    name: string;
    summary: string;
  };
  dataModel: {
    entities: Array<{
      name: string;
      fields: Array<{
        name: string;
        type: AppSpecFieldType;
        required: boolean;
      }>;
    }>;
  };
  views: Array<{
    id: string;
    type: "dashboard" | "list" | "detail" | "form";
    title: string;
    entity?: string;
  }>;
  actions: Array<{
    id: string;
    type: "create" | "update" | "delete" | "read" | "custom";
    title: string;
    entity?: string;
    requiresApproval?: boolean;
  }>;
  policies: Array<{
    id: string;
    rule: string;
    level: "hard" | "soft";
  }>;
  integrations: Array<{
    id: string;
    provider: string;
    purpose: string;
    required: boolean;
  }>;
  tests: Array<{
    id: string;
    type: "unit" | "integration" | "e2e" | "policy";
    description: string;
  }>;
  meta: {
    source: "heuristic_v1" | "assistant_payload";
    confidence: number;
    generated_at: string;
    prompt_excerpt?: string;
    validation?: {
      status: "strict" | "fallback";
      issues?: string[];
    };
  };
}

const COMMON_APP_SUBPATHS = ["database/", "server/", "client/"];

/**
 * WebviewViewProvider for the Sentinel Chat sidebar panel.
 * Implements the Cline-style full sidebar chat experience.
 */
export class SentinelChatViewProvider implements vscode.WebviewViewProvider {
  public static readonly viewId = "sentinel-chat";

  private view?: vscode.WebviewView;
  private activeStreamId: string | null = null;
  private warnedMissingRuntimeTools = false;
  private augmentSettings: AugmentRuntimeSettings = DEFAULT_AUGMENT_SETTINGS;
  private pendingWritePlans: Map<string, PendingWritePlanEntry[]> = new Map();
  private lastAppSpecDraft: AppSpecPayload | null = null;

  constructor(
    private extensionUri: vscode.Uri,
    private client: MCPClient,
    private outputChannel: vscode.OutputChannel,
    private context: vscode.ExtensionContext,
    private onAugmentSettingsChanged: (
      settings: AugmentRuntimeSettings,
    ) => Promise<void>,
  ) {}

  resolveWebviewView(
    webviewView: vscode.WebviewView,
    _context: vscode.WebviewViewResolveContext,
    _token: vscode.CancellationToken,
  ): void {
    this.view = webviewView;
    this.augmentSettings = this.context.globalState.get<AugmentRuntimeSettings>(
      "sentinel.augmentSettings",
      DEFAULT_AUGMENT_SETTINGS,
    );

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
      void this.handleWebviewMessage(msg).catch((err: any) => {
        const message = err?.message ?? String(err);
        this.outputChannel.appendLine(`Webview message handling failed: ${message}`);
        this.postMessage({
          type: "policyActionResult",
          kind: "webview_message_error",
          ok: false,
          message,
        });
      });
    });

    // Notify webview of connection status
    if (this.client.connected) {
      this.postMessage({ type: "connected" });
      this.postMessage({
        type: "augmentSettingsUpdate",
        settings: this.augmentSettings,
      });
      void this.refreshGoalSnapshot();
      void this.refreshRuntimePolicySnapshot();
      void this.refreshRuntimeCapabilitiesSnapshot();
      void this.refreshAlignmentSnapshot();
    }

    this.client.on("connected", () => {
      this.postMessage({ type: "connected" });
      this.postMessage({
        type: "augmentSettingsUpdate",
        settings: this.augmentSettings,
      });
      void this.refreshGoalSnapshot();
      void this.refreshRuntimePolicySnapshot();
      void this.refreshRuntimeCapabilitiesSnapshot();
      void this.refreshAlignmentSnapshot();
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

  private summarizeForTimeline(value: unknown, maxLen: number = 200): string {
    if (value === undefined || value === null) return "null";
    let serialized: string;
    if (typeof value === "string") {
      serialized = value;
    } else {
      try {
        serialized = JSON.stringify(value);
      } catch {
        serialized = String(value);
      }
    }
    if (serialized.length <= maxLen) return serialized;
    return `${serialized.slice(0, maxLen)}...`;
  }

  private async callToolTracked(
    name: string,
    args: Record<string, unknown>,
    turnId?: string,
  ): Promise<unknown> {
    this.emitTimeline("tool", `Tool call: ${name}`, this.summarizeForTimeline(args, 140), turnId);
    try {
      const result = await this.client.callTool(name, args);
      this.emitTimeline(
        "result",
        `Tool result: ${name}`,
        this.summarizeForTimeline(result, 180),
        turnId,
      );
      return result;
    } catch (err: any) {
      this.emitTimeline("error", `Tool failed: ${name}`, err?.message ?? String(err), turnId);
      throw err;
    }
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
        await this.handleFileApproval(msg);
        break;

      case "applySafeWritePlan":
        if (typeof msg.messageId === "string") {
          await this.handleApplySafeWritePlan(msg.messageId);
        }
        break;

      case "mcpRequest":
        try {
          this.outputChannel.appendLine(
            `Executing MCP request from webview: ${msg.method} (${msg.params?.name})`,
          );
          let result;
          if (msg.method === "tools/call") {
            result = await this.callToolTracked(
              msg.params.name,
              msg.params.arguments || {},
              msg.id,
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
      case "setAugmentSettings":
        await this.handleSetAugmentSettings(msg.settings);
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
      case "runQualityHarness":
        await this.handleRunQualityHarness();
        break;

      case "webviewReady":
        if (this.client.connected) {
          this.postMessage({ type: "connected" });
          void this.refreshGoalSnapshot();
          void this.refreshRuntimePolicySnapshot();
          void this.refreshRuntimeCapabilitiesSnapshot();
          void this.refreshAlignmentSnapshot();
        } else {
          this.postMessage({ type: "disconnected" });
        }
        break;

      case "codexLogin":
        vscode.commands.executeCommand(CMD_CODEX_LOGIN);
        break;
    }
  }

  private getWorkspaceRoot(): string {
    return vscode.workspace.workspaceFolders?.[0]?.uri.fsPath ?? process.cwd();
  }

  private resolveWorkspacePath(relativePath: string): string {
    const root = path.resolve(this.getWorkspaceRoot());
    const normalizedRelative = relativePath.replace(/^\/+/, "");
    const resolved = path.resolve(root, normalizedRelative);
    if (resolved !== root && !resolved.startsWith(`${root}${path.sep}`)) {
      throw new Error(`Unsafe write path blocked: ${relativePath}`);
    }
    return resolved;
  }

  private inferLanguage(pathHint: string | undefined, title: string): string {
    const lowerTitle = title.toLowerCase();
    const lowerPath = (pathHint ?? "").toLowerCase();

    if (lowerPath.endsWith(".sql") || lowerTitle.includes("database")) return "sql";
    if (lowerPath.endsWith(".tsx")) return "tsx";
    if (lowerPath.endsWith(".ts")) return "ts";
    if (lowerPath.endsWith(".jsx")) return "jsx";
    if (lowerPath.endsWith(".js") || lowerTitle.includes("backend")) return "js";
    if (lowerTitle.includes("frontend") || lowerTitle.includes("react")) return "jsx";
    if (
      lowerTitle.includes("struttura") ||
      lowerTitle.includes("setup") ||
      lowerTitle.includes("run") ||
      lowerTitle.includes("comandi")
    ) {
      return "bash";
    }
    return "text";
  }

  private inferPath(pathHint: string | undefined, title: string): string | null {
    if (pathHint && pathHint.trim().length > 0) return pathHint.trim();
    const lowerTitle = title.toLowerCase();
    if (lowerTitle.includes("database")) return "database/init.sql";
    if (lowerTitle.includes("backend")) return "server/index.js";
    if (lowerTitle.includes("frontend")) return "client/src/App.jsx";
    return null;
  }

  private detectProjectBaseDir(content: string): string | null {
    const mkdirMatch = content.match(/mkdir\s+-p\s+([a-zA-Z0-9._-]+)\/\{/);
    if (mkdirMatch?.[1]) return mkdirMatch[1];

    const cdMatch = content.match(/cd\s+([a-zA-Z0-9._-]+)\b/);
    if (cdMatch?.[1]) return cdMatch[1];

    return null;
  }

  private withProjectBaseDir(filePath: string, baseDir: string | null): string {
    if (!baseDir) return filePath;
    const normalized = filePath.replace(/^\/+/, "");
    if (normalized.startsWith(`${baseDir}/`)) return normalized;
    if (!COMMON_APP_SUBPATHS.some((prefix) => normalized.startsWith(prefix))) {
      return normalized;
    }
    return `${baseDir}/${normalized}`;
  }

  private splitNumberedSections(content: string): Array<{
    headingLine: string;
    title: string;
    pathHint?: string;
    body: string;
  }> {
    const headingRegex = /^(?:##\s*)?\d+\)\s+[^\n]+$/gm;
    const matches = Array.from(content.matchAll(headingRegex));
    if (matches.length === 0) return [];

    const sections: Array<{
      headingLine: string;
      title: string;
      pathHint?: string;
      body: string;
    }> = [];

    for (let index = 0; index < matches.length; index += 1) {
      const current = matches[index];
      const next = matches[index + 1];
      const start = current.index ?? 0;
      const end = next?.index ?? content.length;
      const chunk = content.slice(start, end).trim();
      const [headingLine, ...restLines] = chunk.split(/\r?\n/);
      const body = restLines.join("\n").trim();
      const titleWithPath = headingLine.replace(/^(?:##\s*)?\d+\)\s*/, "").trim();
      const pathMatch = titleWithPath.match(/`([^`]+)`/);
      const plainPathMatch = titleWithPath.match(/\(([^)]+\.[^)]+)\)/);
      const pathHint = pathMatch?.[1] ?? plainPathMatch?.[1];
      const title = titleWithPath
        .replace(/\s*\(`[^`]+`\)\s*$/, "")
        .replace(/\s*\(([^)]+\.[^)]+)\)\s*$/, "")
        .replace(/\s*`[^`]+`\s*$/, "")
        .trim();

      sections.push({
        headingLine: headingLine.trim(),
        title,
        pathHint,
        body,
      });
    }

    return sections;
  }

  private chunkText(content: string, maxChunkLength: number = 140): string[] {
    if (!content.trim()) return [];
    const chunks: string[] = [];
    let remaining = content;
    while (remaining.length > maxChunkLength) {
      let splitAt = remaining.lastIndexOf("\n", maxChunkLength);
      if (splitAt < Math.floor(maxChunkLength * 0.5)) {
        splitAt = remaining.lastIndexOf(" ", maxChunkLength);
      }
      if (splitAt < 1) splitAt = maxChunkLength;
      const part = remaining.slice(0, splitAt).trimEnd();
      if (part.length > 0) chunks.push(part);
      remaining = remaining.slice(splitAt).trimStart();
    }
    if (remaining.length > 0) chunks.push(remaining);
    return chunks;
  }

  private toSlug(value: string, fallback: string): string {
    const slug = value
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "_")
      .replace(/^_+|_+$/g, "");
    return slug || fallback;
  }

  private toTitleCase(value: string): string {
    return value
      .replace(/[_-]+/g, " ")
      .replace(/\s+/g, " ")
      .trim()
      .replace(/\b\w/g, (char) => char.toUpperCase());
  }

  private clip(value: string, maxLen: number): string {
    if (value.length <= maxLen) return value;
    return `${value.slice(0, maxLen - 3).trimEnd()}...`;
  }

  private detectEntitiesFromText(text: string): string[] {
    const lower = text.toLowerCase();
    const candidates: Array<{ pattern: RegExp; entity: string }> = [
      { pattern: /\b(todo|task|tasks)\b/, entity: "task" },
      { pattern: /\b(project|projects)\b/, entity: "project" },
      { pattern: /\b(user|users|account|accounts|profile|profiles)\b/, entity: "user" },
      { pattern: /\b(order|orders)\b/, entity: "order" },
      { pattern: /\b(customer|customers|client|clients)\b/, entity: "customer" },
      { pattern: /\b(invoice|invoices)\b/, entity: "invoice" },
      { pattern: /\b(product|products|catalog)\b/, entity: "product" },
      { pattern: /\b(booking|bookings|reservation|reservations)\b/, entity: "booking" },
      { pattern: /\b(ticket|tickets|issue|issues)\b/, entity: "ticket" },
      { pattern: /\b(note|notes)\b/, entity: "note" },
    ];

    const found = candidates
      .filter((entry) => entry.pattern.test(lower))
      .map((entry) => entry.entity);
    if (found.length === 0) return ["item"];
    return Array.from(new Set(found)).slice(0, 3);
  }

  private inferAppName(intent: string): string {
    const quoted = intent.match(/"([^"]{3,80})"/)?.[1]?.trim();
    if (quoted) {
      return this.clip(this.toTitleCase(quoted), 48);
    }

    const cleaned = intent
      .replace(/^\/\w+\s*/g, "")
      .replace(/^(build|create|make|implement|generate)\s+/i, "")
      .replace(/\b(app|application|web app|platform)\b/gi, "")
      .trim();
    if (cleaned.length > 0) {
      return this.clip(this.toTitleCase(cleaned), 48);
    }
    return "Sentinel Generated App";
  }

  private inferIntegrations(text: string): AppSpecPayload["integrations"] {
    const lower = text.toLowerCase();
    const integrations: AppSpecPayload["integrations"] = [];
    const push = (provider: string, purpose: string, required: boolean) => {
      integrations.push({
        id: this.toSlug(provider, "integration"),
        provider,
        purpose,
        required,
      });
    };

    if (/\b(postgres|postgresql|mysql|sqlite|mongodb|redis)\b/.test(lower)) {
      push("database", "Persistent data storage", true);
    }
    if (/\b(stripe|payments?)\b/.test(lower)) {
      push("stripe", "Payment processing", false);
    }
    if (/\b(auth0|clerk|oauth|jwt|authentication|auth)\b/.test(lower)) {
      push("auth", "Identity and access management", true);
    }
    if (/\b(sendgrid|mailgun|email|smtp|newsletter)\b/.test(lower)) {
      push("email", "Transactional notifications", false);
    }
    if (/\b(slack|discord|teams|webhook)\b/.test(lower)) {
      push("notifications", "Operational notifications", false);
    }

    return integrations;
  }

  private buildHeuristicAppSpec(
    intentText: string,
    answerContent: string,
    fileOperations?:
      | Array<{
          path: string;
          type: "create" | "edit" | "delete";
          linesAdded: number;
          linesRemoved: number;
          diff: string;
        }>
      | undefined,
  ): AppSpecPayload {
    const combinedText = `${intentText}\n${answerContent}`;
    const entities = this.detectEntitiesFromText(combinedText);
    const appName = this.inferAppName(intentText);

    const dataEntities = entities.map((entity) => {
      const baseFields: AppSpecPayload["dataModel"]["entities"][number]["fields"] = [
        { name: "id", type: "string", required: true },
        { name: "title", type: "string", required: true },
        { name: "status", type: "string", required: true },
        { name: "created_at", type: "date", required: true },
      ];
      if (entity === "user") {
        baseFields.push(
          { name: "email", type: "string", required: true },
          { name: "role", type: "enum", required: true },
        );
      }
      if (entity === "order" || entity === "invoice") {
        baseFields.push(
          { name: "amount", type: "number", required: true },
          { name: "currency", type: "string", required: true },
        );
      }
      return {
        name: entity,
        fields: baseFields,
      };
    });

    const primaryEntity = entities[0];
    let confidence = 0.54;
    if ((fileOperations?.length ?? 0) > 0) confidence += 0.14;
    if (entities.length > 1) confidence += 0.08;
    if (/\b(next\.?js|react|vue|svelte|express|fastapi|django|rails)\b/i.test(combinedText)) {
      confidence += 0.06;
    }
    confidence = Math.min(0.9, Number(confidence.toFixed(2)));

    return {
      version: "1.0",
      app: {
        name: appName,
        summary: this.clip(
          `Draft generated from the current intent and latest response. Primary entity: ${primaryEntity}.`,
          140,
        ),
      },
      dataModel: {
        entities: dataEntities,
      },
      views: [
        { id: "dashboard", type: "dashboard", title: "Operational Dashboard" },
        { id: `${primaryEntity}_list`, type: "list", title: `${this.toTitleCase(primaryEntity)} List`, entity: primaryEntity },
        { id: `${primaryEntity}_form`, type: "form", title: `${this.toTitleCase(primaryEntity)} Form`, entity: primaryEntity },
      ],
      actions: [
        { id: `create_${primaryEntity}`, type: "create", title: `Create ${primaryEntity}`, entity: primaryEntity, requiresApproval: true },
        { id: `update_${primaryEntity}`, type: "update", title: `Update ${primaryEntity}`, entity: primaryEntity, requiresApproval: true },
        { id: `delete_${primaryEntity}`, type: "delete", title: `Delete ${primaryEntity}`, entity: primaryEntity, requiresApproval: true },
        { id: `read_${primaryEntity}`, type: "read", title: `Read ${primaryEntity}`, entity: primaryEntity, requiresApproval: false },
      ],
      policies: [
        { id: "policy_contract", rule: "Public contracts are preserved unless explicit migration is provided.", level: "hard" },
        { id: "policy_determinism", rule: "Execution remains deterministic and bounded by explicit approvals.", level: "hard" },
        { id: "policy_visibility", rule: "Default responses remain outcome-first; internals are progressive details.", level: "soft" },
      ],
      integrations: this.inferIntegrations(combinedText),
      tests: [
        { id: "test_guardrail_existing", type: "integration", description: "Existing behavior remains unchanged for current user flows." },
        { id: "test_regression_write_gate", type: "policy", description: "Sensitive file operations require explicit approval." },
        { id: "test_appspec_smoke", type: "e2e", description: "Draft AppSpec generates a valid preview without runtime errors." },
      ],
      meta: {
        source: "heuristic_v1",
        confidence,
        generated_at: new Date().toISOString(),
        prompt_excerpt: this.clip(intentText.trim(), 140),
        validation: {
          status: "fallback",
        },
      },
    };
  }

  private isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === "object" && value !== null && !Array.isArray(value);
  }

  private safeJsonParse(raw: string): unknown | null {
    try {
      return JSON.parse(raw);
    } catch {
      return null;
    }
  }

  private normalizeFieldType(raw: unknown): AppSpecFieldType {
    if (raw === "string" || raw === "number" || raw === "boolean" || raw === "date" || raw === "enum") {
      return raw;
    }
    return "string";
  }

  private extractAppSpecCodeBlock(content: string): string | null {
    const appspecBlock = content.match(/```appspec\s*([\s\S]*?)```/i);
    if (appspecBlock?.[1]) {
      return appspecBlock[1].trim();
    }

    const jsonBlocks = Array.from(content.matchAll(/```json\s*([\s\S]*?)```/gi));
    for (const block of jsonBlocks) {
      const text = String(block[1] ?? "").trim();
      if (!text) continue;
      if (/"dataModel"\s*:/.test(text) && /"views"\s*:/.test(text) && /"actions"\s*:/.test(text)) {
        return text;
      }
    }

    return null;
  }

  private stripAppSpecCodeBlocks(content: string): string {
    const stripped = content
      .replace(/```appspec[\s\S]*?```/gi, "")
      .replace(/```json[\s\S]*?```/gi, (block) => {
        if (/"dataModel"\s*:/.test(block) && /"views"\s*:/.test(block) && /"actions"\s*:/.test(block)) {
          return "";
        }
        return block;
      })
      .replace(/\n{3,}/g, "\n\n")
      .trim();

    return stripped.length > 0 ? stripped : content;
  }

  private sanitizeAssistantAppSpec(
    candidate: unknown,
    intentText: string,
    answerContent: string,
  ): AppSpecPayload | null {
    if (!this.isRecord(candidate)) return null;

    const issues: string[] = [];
    const appRaw = this.isRecord(candidate.app) ? candidate.app : null;
    if (!appRaw) return null;

    const appName = typeof appRaw.name === "string"
      ? this.clip(appRaw.name.trim(), 64)
      : this.inferAppName(intentText);
    if (appName.length === 0) return null;

    const appSummary = typeof appRaw.summary === "string"
      ? this.clip(appRaw.summary.trim(), 180)
      : this.clip(`Assistant payload for ${appName}.`, 180);

    const dataModelRaw = this.isRecord(candidate.dataModel) ? candidate.dataModel : null;
    const rawEntities = Array.isArray(dataModelRaw?.entities) ? dataModelRaw.entities : [];
    if (rawEntities.length === 0) return null;

    const entities: AppSpecPayload["dataModel"]["entities"] = rawEntities
      .map((entry, idx) => {
        if (!this.isRecord(entry)) return null;
        const name = typeof entry.name === "string"
          ? this.toSlug(entry.name, `entity_${idx + 1}`)
          : `entity_${idx + 1}`;
        const rawFields = Array.isArray(entry.fields) ? entry.fields : [];
        const fields = rawFields
          .map((field, fieldIdx) => {
            if (!this.isRecord(field)) return null;
            const fieldName = typeof field.name === "string"
              ? this.toSlug(field.name, `field_${fieldIdx + 1}`)
              : `field_${fieldIdx + 1}`;
            const fieldType = this.normalizeFieldType(field.type);
            const required = typeof field.required === "boolean" ? field.required : true;
            return {
              name: fieldName,
              type: fieldType,
              required,
            };
          })
          .filter((field): field is { name: string; type: AppSpecFieldType; required: boolean } => Boolean(field));

        if (fields.length === 0) {
          issues.push(`Entity ${name} had no valid fields; injected default field.`);
          fields.push({ name: "id", type: "string", required: true });
        }

        return { name, fields };
      })
      .filter((entity): entity is { name: string; fields: Array<{ name: string; type: AppSpecFieldType; required: boolean }> } => Boolean(entity));

    if (entities.length === 0) return null;
    const primaryEntity = entities[0].name;

    const viewsRaw = Array.isArray(candidate.views) ? candidate.views : [];
    const views: AppSpecPayload["views"] = viewsRaw
      .map((entry, idx) => {
        if (!this.isRecord(entry)) return null;
        const id = typeof entry.id === "string" ? this.toSlug(entry.id, `view_${idx + 1}`) : `view_${idx + 1}`;
        const rawType = typeof entry.type === "string" ? entry.type : "list";
        const type = rawType === "dashboard" || rawType === "list" || rawType === "detail" || rawType === "form"
          ? rawType
          : "list";
        const title = typeof entry.title === "string"
          ? this.clip(entry.title.trim(), 80)
          : this.toTitleCase(`${type} ${primaryEntity}`);
        const entity = typeof entry.entity === "string" ? this.toSlug(entry.entity, primaryEntity) : undefined;
        return { id, type, title, entity };
      })
      .filter((view): view is { id: string; type: "dashboard" | "list" | "detail" | "form"; title: string; entity?: string } => Boolean(view));

    if (views.length === 0) {
      issues.push("Views missing; injected default list/form views.");
      views.push(
        { id: `${primaryEntity}_list`, type: "list", title: `${this.toTitleCase(primaryEntity)} List`, entity: primaryEntity },
        { id: `${primaryEntity}_form`, type: "form", title: `${this.toTitleCase(primaryEntity)} Form`, entity: primaryEntity },
      );
    }

    const actionsRaw = Array.isArray(candidate.actions) ? candidate.actions : [];
    const actions: AppSpecPayload["actions"] = actionsRaw
      .map((entry, idx) => {
        if (!this.isRecord(entry)) return null;
        const id = typeof entry.id === "string" ? this.toSlug(entry.id, `action_${idx + 1}`) : `action_${idx + 1}`;
        const rawType = typeof entry.type === "string" ? entry.type : "custom";
        const type = rawType === "create" || rawType === "update" || rawType === "delete" || rawType === "read" || rawType === "custom"
          ? rawType
          : "custom";
        const title = typeof entry.title === "string"
          ? this.clip(entry.title.trim(), 80)
          : this.toTitleCase(`${type} ${primaryEntity}`);
        const entity = typeof entry.entity === "string" ? this.toSlug(entry.entity, primaryEntity) : primaryEntity;
        const requiresApproval = typeof entry.requiresApproval === "boolean" ? entry.requiresApproval : type !== "read";
        return { id, type, title, entity, requiresApproval };
      })
      .filter((action): action is { id: string; type: "create" | "update" | "delete" | "read" | "custom"; title: string; entity?: string; requiresApproval?: boolean } => Boolean(action));

    if (actions.length === 0) {
      issues.push("Actions missing; injected CRUD defaults.");
      actions.push(
        { id: `create_${primaryEntity}`, type: "create", title: `Create ${primaryEntity}`, entity: primaryEntity, requiresApproval: true },
        { id: `update_${primaryEntity}`, type: "update", title: `Update ${primaryEntity}`, entity: primaryEntity, requiresApproval: true },
        { id: `delete_${primaryEntity}`, type: "delete", title: `Delete ${primaryEntity}`, entity: primaryEntity, requiresApproval: true },
        { id: `read_${primaryEntity}`, type: "read", title: `Read ${primaryEntity}`, entity: primaryEntity, requiresApproval: false },
      );
    }

    const policiesRaw = Array.isArray(candidate.policies) ? candidate.policies : [];
    const policies: AppSpecPayload["policies"] = policiesRaw
      .map((entry, idx) => {
        if (!this.isRecord(entry)) return null;
        const id = typeof entry.id === "string" ? this.toSlug(entry.id, `policy_${idx + 1}`) : `policy_${idx + 1}`;
        const rule = typeof entry.rule === "string" ? this.clip(entry.rule.trim(), 180) : "";
        if (!rule) return null;
        const rawLevel = entry.level;
        const level = rawLevel === "hard" || rawLevel === "soft" ? rawLevel : "soft";
        return { id, rule, level };
      })
      .filter((policy): policy is { id: string; rule: string; level: "hard" | "soft" } => Boolean(policy));

    if (policies.length === 0) {
      policies.push(
        { id: "policy_contract", rule: "Public contracts are preserved unless explicit migration is provided.", level: "hard" },
        { id: "policy_determinism", rule: "Execution remains deterministic and bounded by explicit approvals.", level: "hard" },
      );
    }

    const integrationsRaw = Array.isArray(candidate.integrations) ? candidate.integrations : [];
    const integrations: AppSpecPayload["integrations"] = integrationsRaw
      .map((entry, idx) => {
        if (!this.isRecord(entry)) return null;
        const provider = typeof entry.provider === "string" ? this.clip(entry.provider.trim(), 40) : "";
        if (!provider) return null;
        const purpose = typeof entry.purpose === "string" ? this.clip(entry.purpose.trim(), 120) : "Integration";
        const required = typeof entry.required === "boolean" ? entry.required : false;
        return {
          id: typeof entry.id === "string" ? this.toSlug(entry.id, `integration_${idx + 1}`) : this.toSlug(provider, `integration_${idx + 1}`),
          provider,
          purpose,
          required,
        };
      })
      .filter((integration): integration is { id: string; provider: string; purpose: string; required: boolean } => Boolean(integration));

    const testsRaw = Array.isArray(candidate.tests) ? candidate.tests : [];
    const tests: AppSpecPayload["tests"] = testsRaw
      .map((entry, idx) => {
        if (!this.isRecord(entry)) return null;
        const rawType = typeof entry.type === "string" ? entry.type : "integration";
        const type = rawType === "unit" || rawType === "integration" || rawType === "e2e" || rawType === "policy"
          ? rawType
          : "integration";
        const description = typeof entry.description === "string" ? this.clip(entry.description.trim(), 160) : "";
        if (!description) return null;
        const id = typeof entry.id === "string" ? this.toSlug(entry.id, `test_${idx + 1}`) : `test_${idx + 1}`;
        return { id, type, description };
      })
      .filter((test): test is { id: string; type: "unit" | "integration" | "e2e" | "policy"; description: string } => Boolean(test));

    if (tests.length === 0) {
      tests.push({ id: "test_guardrail_existing", type: "integration", description: "Existing behavior remains unchanged for current user flows." });
    }

    const metaRaw = this.isRecord(candidate.meta) ? candidate.meta : null;
    const confidenceRaw = typeof metaRaw?.confidence === "number" ? metaRaw.confidence : 0.78;
    const confidence = Math.max(0, Math.min(1, Number(confidenceRaw.toFixed(2))));

    return {
      version: "1.0",
      app: {
        name: appName,
        summary: appSummary,
      },
      dataModel: { entities },
      views,
      actions,
      policies,
      integrations,
      tests,
      meta: {
        source: "assistant_payload",
        confidence,
        generated_at: new Date().toISOString(),
        prompt_excerpt: this.clip(intentText.trim(), 140),
        validation: {
          status: "strict",
          issues: issues.length > 0 ? issues : undefined,
        },
      },
    };
  }

  private buildAppSpecRefinePrompt(spec: AppSpecPayload): string {
    return [
      "Refine the current AppSpec while preserving contracts and deterministic behavior.",
      "Return:",
      "1) Outcome-first sections: What I changed / What needs your approval / What happens next.",
      "2) A strict JSON block fenced as ```appspec``` with keys:",
      "   app, dataModel.entities, views, actions, policies, integrations, tests, meta.",
      "No prose after the appspec block.",
      "",
      "Current AppSpec:",
      "```appspec",
      JSON.stringify(spec, null, 2),
      "```",
    ].join("\n");
  }

  private buildAppSpecPlanPrompt(spec: AppSpecPayload): string {
    return [
      "Generate a minimal, zero-regression implementation plan from this AppSpec.",
      "Constraints:",
      "- contract-preserving, deterministic, bounded, reversible",
      "- minimal blast radius",
      "- include verification tests and rollback notes",
      "Return only outcome-first sections and no code.",
      "",
      "AppSpec:",
      "```appspec",
      JSON.stringify(spec, null, 2),
      "```",
    ].join("\n");
  }

  private buildChatPlan(content: string): ParsedChatPlan | null {
    const sections = this.splitNumberedSections(content);
    if (sections.length === 0) return null;
    const projectBaseDir = this.detectProjectBaseDir(content);

    const uiSections: ChatSectionPayload[] = [];
    const fileOperations: Array<{
      path: string;
      type: "create" | "edit" | "delete";
      linesAdded: number;
      linesRemoved: number;
      diff: string;
    }> = [];
    const writeEntries: PendingWritePlanEntry[] = [];
    let hadUnfencedSection = false;

    const normalizedSections = sections.map((section, idx) => {
      const body = section.body.trim();
      const language = this.inferLanguage(section.pathHint, section.title);
      const firstCodeBlock = body.match(/```([a-zA-Z0-9_-]*)\n([\s\S]*?)```/m);
      const code = firstCodeBlock
        ? firstCodeBlock[2].trim()
        : body;
      const effectiveLanguage = firstCodeBlock?.[1]?.trim() || language;
      const rawInferredPath = this.inferPath(section.pathHint, section.title);
      const inferredPath = rawInferredPath
        ? this.withProjectBaseDir(rawInferredPath, projectBaseDir)
        : null;

      if (body.length > 0) {
        uiSections.push({
          id: `${idx + 1}-${section.title.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`,
          title: section.title,
          pathHint: inferredPath ?? section.pathHint,
          language: effectiveLanguage,
          content: code,
        });
      }

      if (inferredPath && code.length > 0) {
        const lineCount = code.split(/\r?\n/).length;
        const diff = code
          .split(/\r?\n/)
          .map((line) => `+${line}`)
          .join("\n");

        writeEntries.push({
          id: `${idx + 1}-${inferredPath.replace(/[^a-zA-Z0-9]+/g, "-")}`,
          title: section.title,
          pathHint: inferredPath,
          language: effectiveLanguage,
          content: code,
          path: inferredPath,
          applied: false,
        });

        fileOperations.push({
          path: inferredPath,
          type: "create",
          linesAdded: lineCount,
          linesRemoved: 0,
          diff,
        });
      }

      if (!/```/.test(body) && body.length > 0) {
        hadUnfencedSection = true;
      }

      if (body.length === 0) {
        return section.headingLine;
      }

      if (/```/.test(body)) {
        return `${section.headingLine}\n\n${section.body}`.trim();
      }

      return `${section.headingLine}\n\n\`\`\`${language}\n${body}\n\`\`\``;
    });

    let normalizedContent = content;
    if (hadUnfencedSection) {
      const firstHeading = content.search(/^(?:##\s*)?\d+\)/m);
      const prefix = firstHeading > 0 ? content.slice(0, firstHeading).trim() : "";
      normalizedContent = [prefix, ...normalizedSections]
        .filter((part) => part.trim().length > 0)
        .join("\n\n")
        .trim();
    }

    return {
      normalizedContent,
      sections: uiSections,
      fileOperations,
      writeEntries,
    };
  }

  private async applyWriteEntry(
    messageId: string,
    entry: PendingWritePlanEntry,
  ): Promise<boolean> {
    try {
      this.emitTimeline("approval", "safe_write approval", entry.path, messageId);
      const scan = (await this.callToolTracked(
        "safe_write",
        { path: entry.path, content: entry.content },
        messageId,
      )) as any;
      const isSafe = scan?.is_safe !== false;
      if (!isSafe) {
        const threats = Array.isArray(scan?.threats)
          ? scan.threats
              .map((threat: any) =>
                typeof threat === "string"
                  ? threat
                  : (threat?.description ?? JSON.stringify(threat)),
              )
              .join("; ")
          : "security scan failed";
        const error = `Blocked by safe_write: ${threats}`;
        this.postMessage({
          type: "fileApprovalResult",
          messageId,
          path: entry.path,
          approved: false,
          error,
        });
        this.emitTimeline("error", "safe_write blocked file", error, messageId);
        return false;
      }

      const absolutePath = this.resolveWorkspacePath(entry.path);
      await fs.mkdir(path.dirname(absolutePath), { recursive: true });
      await fs.writeFile(absolutePath, entry.content, "utf8");
      entry.applied = true;

      this.postMessage({
        type: "fileApprovalResult",
        messageId,
        path: entry.path,
        approved: true,
      });
      this.emitTimeline("result", "File written", entry.path, messageId);
      return true;
    } catch (err: any) {
      const error = err?.message ?? String(err);
      this.postMessage({
        type: "fileApprovalResult",
        messageId,
        path: entry.path,
        approved: false,
        error,
      });
      this.emitTimeline("error", "File write failed", `${entry.path}: ${error}`, messageId);
      return false;
    }
  }

  private async handleFileApproval(msg: any): Promise<void> {
    const messageId = typeof msg?.messageId === "string" ? msg.messageId : "";
    const filePath = typeof msg?.path === "string" ? msg.path : "";
    const approved = Boolean(msg?.approved);
    if (!messageId || !filePath) return;

    if (!approved) {
      this.postMessage({
        type: "fileApprovalResult",
        messageId,
        path: filePath,
        approved: false,
        error: "Rejected by user",
      });
      this.emitTimeline("cancel", "File write rejected", filePath, messageId);
      return;
    }

    const plan = this.pendingWritePlans.get(messageId) ?? [];
    const entry = plan.find((item) => item.path === filePath);
    if (!entry) {
      this.postMessage({
        type: "fileApprovalResult",
        messageId,
        path: filePath,
        approved: false,
        error: "No pending write plan for this file.",
      });
      this.emitTimeline("error", "Missing write plan", filePath, messageId);
      return;
    }

    const ok = await this.applyWriteEntry(messageId, entry);
    if (ok && plan.every((item) => item.applied)) {
      this.pendingWritePlans.delete(messageId);
    }
  }

  private async handleApplySafeWritePlan(messageId: string): Promise<void> {
    const plan = this.pendingWritePlans.get(messageId) ?? [];
    if (plan.length === 0) {
      this.postMessage({
        type: "policyActionResult",
        kind: "safe_write_plan",
        ok: false,
        message: "No pending safe_write plan for this turn.",
      });
      return;
    }

    let success = 0;
    let failed = 0;
    for (const entry of plan) {
      if (entry.applied) continue;
      const ok = await this.applyWriteEntry(messageId, entry);
      if (ok) success += 1;
      else failed += 1;
    }

    this.postMessage({
      type: "policyActionResult",
      kind: "safe_write_plan",
      ok: failed === 0,
      message: `safe_write plan completed: ${success} applied, ${failed} blocked.`,
    });
    if (failed === 0) {
      this.pendingWritePlans.delete(messageId);
    }
    this.emitTimeline(
      failed === 0 ? "result" : "error",
      "safe_write plan completed",
      `${success} applied, ${failed} blocked`,
      messageId,
    );
  }

  private async buildExecuteFirstPendingPrompt(messageId: string): Promise<string | null> {
    if (!(await this.client.supportsTool("get_goal_graph"))) {
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: "Tool get_goal_graph non disponibile: impossibile risolvere il primo goal pending.",
      });
      this.emitTimeline("error", "Slash command failed", "get_goal_graph missing", messageId);
      return null;
    }
    if (!(await this.client.supportsTool("chat"))) {
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: "Tool chat non disponibile nel backend corrente.",
      });
      this.emitTimeline("error", "Slash command failed", "chat missing", messageId);
      return null;
    }

    const graph: any = await this.callToolTracked("get_goal_graph", {}, messageId);
    const nodes = Array.isArray(graph?.nodes) ? graph.nodes : [];
    const edges = Array.isArray(graph?.edges) ? graph.edges : [];
    const pendingNodes = nodes.filter(
      (node: any) =>
        node?.id !== "root" &&
        String(node?.data?.status ?? "").toLowerCase() === "pending",
    );

    if (pendingNodes.length === 0) {
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: "Nessun goal pending rilevato. Apri Forge e verifica lo stato del manifold.",
      });
      this.emitTimeline("result", "No pending goals", "Slash command completed", messageId);
      return null;
    }

    const pendingIds = new Set(pendingNodes.map((node: any) => String(node.id)));
    const inboundPendingDeps = new Map<string, number>();
    for (const node of pendingNodes) {
      inboundPendingDeps.set(String(node.id), 0);
    }
    for (const edge of edges) {
      const source = String(edge?.source ?? "");
      const target = String(edge?.target ?? "");
      if (pendingIds.has(source) && pendingIds.has(target)) {
        inboundPendingDeps.set(target, (inboundPendingDeps.get(target) ?? 0) + 1);
      }
    }

    const orderedPending = [...pendingNodes].sort((a: any, b: any) => {
      const aId = String(a?.id ?? "");
      const bId = String(b?.id ?? "");
      const depCmp = (inboundPendingDeps.get(aId) ?? 0) - (inboundPendingDeps.get(bId) ?? 0);
      if (depCmp !== 0) return depCmp;
      const aValue = Number(a?.data?.value ?? 0);
      const bValue = Number(b?.data?.value ?? 0);
      if (aValue !== bValue) return bValue - aValue;
      return aId.localeCompare(bId);
    });

    const next = orderedPending[0];
    const goalId = String(next?.id ?? "");
    const goalLabel = String(next?.data?.label ?? "").trim();

    const executionPrompt =
      `Implementa SOLO il primo goal pending del manifold.\n` +
      `Goal ID: ${goalId}\n` +
      `Goal descrizione: ${goalLabel || "(vuota)"}\n\n` +
      `Vincoli obbligatori:\n` +
      `- niente scaffolding iniziale\n` +
      `- solo file changes mirati\n` +
      `- test minimi necessari\n` +
      `- comandi di verifica strettamente necessari\n` +
      `- mantieni output con sezioni operative e path espliciti`;

    this.emitTimeline("plan", "First pending goal selected", `${goalId} ${goalLabel}`, messageId);
    return executionPrompt;
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
    const trimmedText = text.trim();
    let effectiveText = text;

    if (trimmedText === "/execute-first-pending") {
      this.emitTimeline(
        "plan",
        "Slash command /execute-first-pending",
        "Resolving first pending goal",
        messageId,
      );
      try {
        const nextPrompt = await this.buildExecuteFirstPendingPrompt(messageId);
        if (!nextPrompt) return;
        effectiveText = nextPrompt;
      } catch (err: any) {
        const error = err?.message ?? String(err);
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: `Errore durante /execute-first-pending: ${error}`,
        });
        this.emitTimeline("error", "Slash command failed", error, messageId);
        return;
      }
    }

    if (trimmedText === "/init") {
      this.emitTimeline("plan", "Slash command /init", "Checking project initialization state", messageId);
      try {
        const graph: any = await this.callToolTracked("get_goal_graph", {}, messageId);
        const nodes = Array.isArray(graph?.nodes) ? graph.nodes : [];
        const rootNode = nodes.find((node: any) => node?.id === "root");
        const rootLabel = String(rootNode?.data?.label ?? "").trim();
        const pendingGoals = nodes.filter(
          (node: any) =>
            node?.id !== "root" &&
            String(node?.data?.status ?? "").toLowerCase() === "pending",
        );

        if (rootLabel.length > 0) {
          this.postMessage({
            type: "chatResponse",
            id: messageId,
            content:
              `✅ Project already initialized.\n` +
              `Root intent: **${rootLabel}**\n` +
              `Pending goals: **${pendingGoals.length}**\n\n` +
              `Next step: run **"Implementa SOLO il primo goal pending in todo-app. Niente scaffolding iniziale. Produci solo file changes + test minimi."**`,
          });
          this.emitTimeline("result", "Initialization state loaded", rootLabel, messageId);
          return;
        }
      } catch {
        // Fall back to usage guidance below.
      }

      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content:
          "Usage: `/init <project description>`\n\n" +
          "Example:\n`/init Build a production-ready full-stack todo app with React + Express + PostgreSQL`",
      });
      this.emitTimeline("result", "Init usage provided", "Missing project description", messageId);
      return;
    }

    if (trimmedText.startsWith("/init ")) {
      const description = trimmedText.replace("/init", "").trim();
      if (!description) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: "Usage: `/init <project description>`",
        });
        return;
      }
      this.emitTimeline("plan", "Slash command /init", description, messageId);
      try {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: `🚀 Initializing project: "${description}"...`,
        });

        const result: any = await this.callToolTracked("init_project", {
          description,
        }, messageId);

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

    if (trimmedText === "/appspec-refine") {
      if (!this.lastAppSpecDraft) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: "No AppSpec draft available yet. Send a normal request first to generate one.",
        });
        this.emitTimeline("error", "Slash command failed", "No AppSpec draft available", messageId);
        return;
      }
      this.emitTimeline("plan", "Slash command /appspec-refine", "Refining current AppSpec", messageId);
      effectiveText = this.buildAppSpecRefinePrompt(this.lastAppSpecDraft);
    }

    if (trimmedText === "/appspec-plan") {
      if (!this.lastAppSpecDraft) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: "No AppSpec draft available yet. Send a normal request first to generate one.",
        });
        this.emitTimeline("error", "Slash command failed", "No AppSpec draft available", messageId);
        return;
      }
      this.emitTimeline("plan", "Slash command /appspec-plan", "Generating implementation plan from AppSpec", messageId);
      effectiveText = this.buildAppSpecPlanPrompt(this.lastAppSpecDraft);
    }

    if (trimmedText === "/help") {
      this.emitTimeline("plan", "Slash command /help", "Help menu requested", messageId);
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content:
          "Comandi disponibili:\n- `/init <descrizione>`\n- `/execute-first-pending`\n- `/appspec-refine`\n- `/appspec-plan`\n- `/clear-memory`\n- `/memory-status`\n- `/memory-search <query>`\n- `/memory-export [path]`\n- `/memory-import <path> [merge=true|false]`",
      });
      return;
    }

    if (trimmedText === "/memory-status") {
      if (!(await this.client.supportsTool("chat_memory_status"))) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: "Tool chat_memory_status non disponibile nel backend corrente.",
        });
        return;
      }
      this.emitTimeline("tool", "Memory status", "Querying memory state", messageId);
      const result = (await this.callToolTracked("chat_memory_status", {}, messageId)) as any;
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: `Memory turns: ${result?.turn_count ?? 0}\nRecent:\n${(result?.recent_turns ?? [])
          .map((t: any) => `- ${t.id?.slice(0, 8)} ${t.intent_summary ?? ""}`)
          .join("\n")}`,
      });
      return;
    }

    if (trimmedText.startsWith("/memory-search ")) {
      if (!(await this.client.supportsTool("chat_memory_search"))) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: "Tool chat_memory_search non disponibile nel backend corrente.",
        });
        return;
      }
      const query = trimmedText.replace("/memory-search ", "").trim();
      this.emitTimeline("tool", "Memory search", query, messageId);
      const result = (await this.callToolTracked("chat_memory_search", {
        query,
        limit: 8,
      }, messageId)) as any;
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: `Memory hits (${result?.count ?? 0}) for "${query}":\n${(result?.hits ?? [])
          .map((h: any) => `- ${h.id?.slice(0, 8)}: ${h.intent_summary ?? ""}`)
          .join("\n")}`,
      });
      return;
    }

    if (trimmedText.startsWith("/memory-export")) {
      if (!(await this.client.supportsTool("chat_memory_export"))) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: "Tool chat_memory_export non disponibile nel backend corrente.",
        });
        return;
      }
      const maybePath = trimmedText.replace("/memory-export", "").trim();
      this.emitTimeline("tool", "Memory export", maybePath || "default path", messageId);
      const args: Record<string, unknown> = {};
      if (maybePath) args.path = maybePath;
      const result = (await this.callToolTracked("chat_memory_export", args, messageId)) as any;
      this.postMessage({
        type: "chatResponse",
        id: messageId,
        content: result?.ok
          ? `Memory export completato: ${result.path}\nTurns: ${result.turn_count}`
          : `Memory export fallito: ${result?.error ?? "unknown error"}`,
      });
      return;
    }

    if (trimmedText.startsWith("/memory-import ")) {
      if (!(await this.client.supportsTool("chat_memory_import"))) {
        this.postMessage({
          type: "chatResponse",
          id: messageId,
          content: "Tool chat_memory_import non disponibile nel backend corrente.",
        });
        return;
      }
      const payload = trimmedText.replace("/memory-import ", "").trim();
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
      const result = (await this.callToolTracked("chat_memory_import", {
        path: pathArg,
        merge,
      }, messageId)) as any;
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
      const result: any = await this.callToolTracked("chat", {
        message: effectiveText
      }, messageId);

      let content = "No response from Sentinel.";
      let thoughtChain: string[] | undefined = undefined;
      let explainability: Record<string, unknown> | undefined = undefined;
      let innovation: Record<string, unknown> | undefined = undefined;
      let appSpec: AppSpecPayload | undefined;
      let structuredAppSpecCandidate: unknown = undefined;
      let streamChunks: string[] = [];
      let sections: ChatSectionPayload[] | undefined;
      let fileOperations:
        | Array<{
            path: string;
            type: "create" | "edit" | "delete";
            linesAdded: number;
            linesRemoved: number;
            diff: string;
          }>
        | undefined;

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
        if (structured.explainability && typeof structured.explainability === "object") {
          explainability = { ...(structured.explainability as Record<string, unknown>) };
        }
        if (structured.innovation && typeof structured.innovation === "object") {
          innovation = { ...(structured.innovation as Record<string, unknown>) };
        }
        if (structured.appspec !== undefined) {
          structuredAppSpecCandidate = structured.appspec;
        } else if (structured.app_spec !== undefined) {
          structuredAppSpecCandidate = structured.app_spec;
        }
        const contextProvider =
          typeof structured.context_provider === "string"
            ? structured.context_provider
            : this.augmentSettings.enabled
              ? "free_stack_primary (augment_secondary)"
              : "free_stack_primary";
        explainability = {
          ...(explainability ?? {}),
          context_provider: contextProvider,
          context_policy_mode: this.augmentSettings.enabled
            ? this.augmentSettings.mode
            : "disabled",
          context_fallback_reason:
            typeof structured.context_fallback_reason === "string"
              ? structured.context_fallback_reason
              : null,
        };
        if (Array.isArray(structured.stream_chunks)) {
          streamChunks = structured.stream_chunks.map((v) => String(v));
        }
      } else if (typeof result === "string") {
        content = result;
      }

      if (structuredAppSpecCandidate !== undefined) {
        const validated = this.sanitizeAssistantAppSpec(
          structuredAppSpecCandidate,
          effectiveText,
          content,
        );
        if (validated) {
          appSpec = validated;
          this.emitTimeline(
            "plan",
            "AppSpec validated",
            `${validated.dataModel.entities.length} entities • strict payload`,
            messageId,
          );
        } else {
          this.emitTimeline("error", "AppSpec payload invalid", "Falling back to safe heuristic", messageId);
        }
      }

      if (!appSpec) {
        const maybeAppSpecBlock = this.extractAppSpecCodeBlock(content);
        if (maybeAppSpecBlock) {
          const parsed = this.safeJsonParse(maybeAppSpecBlock);
          const validated = this.sanitizeAssistantAppSpec(parsed, effectiveText, content);
          if (validated) {
            appSpec = validated;
            content = this.stripAppSpecCodeBlocks(content);
            this.emitTimeline(
              "plan",
              "AppSpec block parsed",
              `${validated.dataModel.entities.length} entities • strict block`,
              messageId,
            );
          } else {
            this.emitTimeline("error", "AppSpec block invalid", "Falling back to safe heuristic", messageId);
          }
        }
      }

      const parsedPlan = this.buildChatPlan(content);
      if (parsedPlan) {
        content = parsedPlan.normalizedContent;
        sections = parsedPlan.sections;
        fileOperations = parsedPlan.fileOperations;
        if (parsedPlan.writeEntries.length > 0) {
          this.pendingWritePlans.set(messageId, parsedPlan.writeEntries);
          this.emitTimeline(
            "plan",
            "safe_write plan prepared",
            `${parsedPlan.writeEntries.length} files pending approval`,
            messageId,
          );
        } else {
          this.pendingWritePlans.delete(messageId);
        }
      } else {
        this.pendingWritePlans.delete(messageId);
      }
      if (!appSpec) {
        appSpec = this.buildHeuristicAppSpec(effectiveText, content, fileOperations);
        this.emitTimeline(
          "plan",
          "AppSpec draft prepared",
          `${appSpec.dataModel.entities.length} entities • confidence ${Math.round(appSpec.meta.confidence * 100)}% • fallback`,
          messageId,
        );
      }
      this.lastAppSpecDraft = appSpec;

      streamChunks = this.chunkText(content, 140);

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
          sections,
          innovation,
          appSpec,
          fileOperations,
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

  private async refreshAlignmentSnapshot(): Promise<void> {
    if (!this.client.connected) return;
    try {
      const alignment = (await this.client.callTool("get_alignment", {})) as any;
      if (alignment && typeof alignment.score === "number") {
        this.postMessage({
          type: "alignmentUpdate",
          score: alignment.score,
          confidence: alignment.confidence ?? 0,
          status: alignment.status ?? "UNKNOWN",
          trend: 0,
        });
      }
    } catch (err: any) {
      this.outputChannel.appendLine(`Failed to refresh alignment: ${err.message}`);
    }
  }

  private async refreshRuntimeCapabilitiesSnapshot(): Promise<void> {
    if (!this.client.connected) return;
    try {
      const toolsResult = await this.client.listTools();
      const tools = Array.isArray((toolsResult as any)?.tools) ? (toolsResult as any).tools : [];
      const toolNames = tools
        .map((tool: any) => (typeof tool?.name === "string" ? tool.name : null))
        .filter((name: string | null): name is string => Boolean(name));

      const serverInfo = this.client.getServerInfo();
      this.postMessage({
        type: "runtimeCapabilities",
        capabilities: {
          tool_count: toolNames.length,
          tools: toolNames,
          server_name: serverInfo?.name ?? "sentinel-server",
          server_version: serverInfo?.version ?? "unknown",
          connected: true,
        },
      });
    } catch (err: any) {
      this.outputChannel.appendLine(`Failed to refresh runtime capabilities: ${err.message}`);
      this.postMessage({
        type: "runtimeCapabilities",
        capabilities: {
          tool_count: 0,
          tools: [],
          server_name: "sentinel-server",
          server_version: "unknown",
          connected: false,
        },
      });
    }
  }

  private async refreshRuntimePolicySnapshot(): Promise<void> {
    if (!this.client.connected) {
      return;
    }

    let supportedTools: Set<string> = new Set();
    try {
      supportedTools = await this.client.listToolNames();
    } catch (err: any) {
      this.outputChannel.appendLine(
        `Failed to discover MCP tools for runtime policy snapshot: ${err.message}`,
      );
    }

    const hasReliability = supportedTools.has("get_reliability");
    const hasGovernance = supportedTools.has("governance_status");
    const hasWorldModel = supportedTools.has("get_world_model");
    const hasQualityStatus = supportedTools.has("get_quality_status");
    if (!this.warnedMissingRuntimeTools && (!hasReliability || !hasGovernance)) {
      this.warnedMissingRuntimeTools = true;
      const missing = [
        !hasReliability ? "get_reliability" : null,
        !hasGovernance ? "governance_status" : null,
      ].filter(Boolean);
      const message =
        `Connected backend does not expose runtime policy tools: ${missing.join(", ")}. ` +
        "Upgrade sentinel-cli (current path may be legacy) to enable Reliability/Governance panels.";
      this.outputChannel.appendLine(message);
      this.postMessage({
        type: "policyActionResult",
        kind: "runtime_capability",
        ok: false,
        message,
      });
    }

    let worldModel: any = null;
    if (hasWorldModel) {
      try {
        worldModel = (await this.client.callTool("get_world_model", {})) as any;
      } catch (err: any) {
        this.outputChannel.appendLine(`Failed to refresh world model: ${err.message}`);
      }
    }

    try {
      if (hasReliability) {
        const reliability = (await this.client.callTool("get_reliability", {})) as any;
        if (reliability && !reliability.error) {
          this.postMessage({
            type: "reliabilityUpdate",
            reliability: reliability.reliability,
            reliability_thresholds: reliability.reliability_thresholds,
            reliability_slo: reliability.reliability_slo,
          });
        }
      }
    } catch (err: any) {
      this.outputChannel.appendLine(`Failed to refresh reliability: ${err.message}`);
    }

    try {
      if (hasGovernance) {
        const governance = (await this.client.callTool("governance_status", {})) as any;
        if (governance && !governance.error) {
          this.postMessage({
            type: "governanceUpdate",
            governance: {
              ...governance,
              world_model: worldModel && !worldModel.error ? worldModel : null,
            },
          });
        }
      }
    } catch (err: any) {
      this.outputChannel.appendLine(`Failed to refresh governance: ${err.message}`);
    }

    try {
      if (hasQualityStatus) {
        const quality = (await this.client.callTool("get_quality_status", {})) as any;
        if (quality && !quality.error) {
          this.postMessage({
            type: "qualityUpdate",
            quality,
          });
        }
      }
    } catch (err: any) {
      this.outputChannel.appendLine(`Failed to refresh quality status: ${err.message}`);
    }
  }

  private async handleSetAugmentSettings(
    raw: Partial<AugmentRuntimeSettings>,
  ): Promise<void> {
    const next: AugmentRuntimeSettings = {
      enabled: Boolean(raw?.enabled),
      mode:
        raw?.mode === "internal_only" || raw?.mode === "byo_customer"
          ? raw.mode
          : "disabled",
      enforceByo: raw?.enforceByo !== false,
    };

    this.augmentSettings = next;
    await this.context.globalState.update("sentinel.augmentSettings", next);
    this.postMessage({ type: "augmentSettingsUpdate", settings: next });

    try {
      await this.onAugmentSettingsChanged(next);
      this.postMessage({
        type: "policyActionResult",
        kind: "augment_settings",
        ok: true,
        message: `Augment MCP settings applied (${next.enabled ? next.mode : "disabled"}).`,
      });
      await this.refreshRuntimePolicySnapshot();
      await this.refreshRuntimeCapabilitiesSnapshot();
    } catch (err: any) {
      this.postMessage({
        type: "policyActionResult",
        kind: "augment_settings",
        ok: false,
        message: err?.message ?? "Failed to apply Augment MCP settings.",
      });
    }
  }

  private async handleGovernanceApprove(note?: string): Promise<void> {
    if (!this.client.connected) return;
    if (!(await this.client.supportsTool("governance_approve"))) {
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_approve",
        ok: false,
        message: "Tool governance_approve non disponibile nel backend corrente.",
      });
      return;
    }
    try {
      this.emitTimeline("approval", "Governance approve requested", note ?? "", undefined);
      const result = (await this.callToolTracked("governance_approve", {
        note: typeof note === "string" ? note : "",
      }, undefined)) as any;
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
    if (!(await this.client.supportsTool("governance_reject"))) {
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_reject",
        ok: false,
        message: "Tool governance_reject non disponibile nel backend corrente.",
      });
      return;
    }
    try {
      this.emitTimeline("approval", "Governance reject requested", reason ?? "", undefined);
      const result = (await this.callToolTracked("governance_reject", {
        reason: typeof reason === "string" ? reason : "Rejected from VSCode UI",
      }, undefined)) as any;
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
    if (!(await this.client.supportsTool("governance_seed"))) {
      this.postMessage({
        type: "policyActionResult",
        kind: "governance_seed",
        ok: false,
        message: "Tool governance_seed non disponibile nel backend corrente.",
      });
      return;
    }
    try {
      this.emitTimeline(
        "tool",
        "Governance seed requested",
        `apply=${apply} lock_required=${lockRequired}`,
        undefined,
      );
      const result = (await this.callToolTracked("governance_seed", {
        apply,
        lock_required: lockRequired,
      }, undefined)) as any;

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
        let governanceStatus: any = {};
        if (await this.client.supportsTool("governance_status")) {
          governanceStatus = await this.client.callTool("governance_status", {});
        }
        this.postMessage({
          type: "governanceUpdate",
          governance: {
            ...governanceStatus,
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
      const result = (await this.callToolTracked("chat_memory_clear", {}, undefined)) as any;
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

  private async handleRunQualityHarness(): Promise<void> {
    if (!this.client.connected) return;
    if (!(await this.client.supportsTool("run_quality_harness"))) {
      this.postMessage({
        type: "policyActionResult",
        kind: "run_quality_harness",
        ok: false,
        message: "Tool run_quality_harness non disponibile nel backend corrente.",
      });
      return;
    }
    try {
      this.emitTimeline("tool", "Quality harness", "run_quality_harness", undefined);
      const result = (await this.callToolTracked("run_quality_harness", {}, undefined)) as any;
      const ok = result?.ok === true;
      this.postMessage({
        type: "policyActionResult",
        kind: "run_quality_harness",
        ok,
        message: ok
          ? "World-class quality harness completed."
          : `Quality harness failed: ${result?.error ?? "unknown error"}`,
      });
      this.postMessage({
        type: "qualityUpdate",
        quality: result?.latest
          ? { ok: true, latest: result.latest }
          : {
              ok: ok,
              latest: null,
              message: "Harness completed but no latest report was parsed.",
            },
      });
      await this.refreshRuntimePolicySnapshot();
    } catch (err: any) {
      this.postMessage({
        type: "policyActionResult",
        kind: "run_quality_harness",
        ok: false,
        message: err.message,
      });
      this.emitTimeline("error", "Quality harness failed", err.message, undefined);
    }
  }
}
