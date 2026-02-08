import React, { useEffect, useMemo, useState } from "react";
import type { ChatMessage } from "../../state/types";
import { renderMarkdown } from "../../utils/markdown";
import ToolCallCard from "./ToolCallCard";
import FileApproval from "../Actions/FileApproval";
import { cn } from "@/lib/utils";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";
import { Button } from "../ui/button";

type OutcomeKey = "changed" | "approval" | "next";

interface OutcomeSummary {
  changed: string[];
  approval: string[];
  next: string[];
}

function normalizeHeading(line: string): string {
  return line
    .toLowerCase()
    .replace(/[`*_]/g, "")
    .replace(/^\s*[#>\-*\d\.\)\(]+\s*/, "")
    .replace(/\s*[:\-]+\s*$/, "")
    .trim();
}

function detectOutcomeHeading(line: string): OutcomeKey | null {
  const normalized = normalizeHeading(line);
  if (
    normalized.startsWith("what i changed") ||
    normalized.startsWith("changes made") ||
    normalized.startsWith("cosa ho cambiato")
  ) {
    return "changed";
  }
  if (
    normalized.startsWith("what needs your approval") ||
    normalized.startsWith("approval needed") ||
    normalized.startsWith("cosa devi approvare")
  ) {
    return "approval";
  }
  if (
    normalized.startsWith("what happens next") ||
    normalized.startsWith("next steps") ||
    normalized.startsWith("cosa succede dopo")
  ) {
    return "next";
  }
  return null;
}

function cleanLine(line: string): string {
  return line
    .replace(/^\s*(?:[-*+]|\d+[.)])\s*/, "")
    .replace(/\s+/g, " ")
    .trim();
}

function clipText(value: string, max: number = 180): string {
  if (value.length <= max) return value;
  return `${value.slice(0, max - 3).trimEnd()}...`;
}

function dedupeAndLimit(values: string[], fallback: string, limit: number = 3): string[] {
  const unique = Array.from(new Set(values.map((item) => item.trim()).filter(Boolean)));
  if (unique.length === 0) return [fallback];
  return unique.slice(0, limit).map((item) => clipText(item));
}

function extractStructuredOutcome(content: string): OutcomeSummary | null {
  const lines = content.split(/\r?\n/);
  const bucket: Record<OutcomeKey, string[]> = {
    changed: [],
    approval: [],
    next: [],
  };
  let current: OutcomeKey | null = null;

  for (const rawLine of lines) {
    const heading = detectOutcomeHeading(rawLine);
    if (heading) {
      current = heading;
      continue;
    }
    if (!current) continue;
    const cleaned = cleanLine(rawLine);
    if (!cleaned) continue;
    bucket[current].push(cleaned);
  }

  if (bucket.changed.length && bucket.approval.length && bucket.next.length) {
    return {
      changed: dedupeAndLimit(bucket.changed, "Changes prepared."),
      approval: dedupeAndLimit(bucket.approval, "No explicit approval required."),
      next: dedupeAndLimit(bucket.next, "Continue with the next task."),
    };
  }

  return null;
}

function deriveOutcome(message: ChatMessage): OutcomeSummary {
  const structured = extractStructuredOutcome(message.content);
  if (structured) return structured;

  const pendingOps =
    message.fileOperations?.filter((operation) => operation.approved === undefined) ?? [];

  const sectionDrivenChanges =
    message.sections?.map((section) =>
      section.pathHint ? `${section.title} (${section.pathHint})` : section.title,
    ) ?? [];

  const contentLines = message.content
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(
      (line) =>
        line.length > 0 &&
        !line.startsWith("```") &&
        !line.startsWith("#") &&
        !detectOutcomeHeading(line),
    );

  const changed = dedupeAndLimit(
    sectionDrivenChanges.length ? sectionDrivenChanges : contentLines.slice(0, 3),
    "Response generated and ready for review.",
  );

  const approval =
    pendingOps.length > 0
      ? dedupeAndLimit(
          pendingOps.map((operation) => `Approve ${operation.type} on ${operation.path}`),
          "Review required approvals.",
        )
      : ["No explicit approval required right now."];

  const next = dedupeAndLimit(
    pendingOps.length > 0
      ? [
          "Approve the pending file plan to apply changes safely.",
          "After approval, ask Sentinel to validate or continue execution.",
        ]
      : [
          "Review the result and request refinements if needed.",
          "If ready, ask Sentinel for tests, verification, or the next implementation step.",
        ],
    "Continue with the next task.",
  );

  return { changed, approval, next };
}

export default function MessageBubble({
  message,
  index,
  compact = false,
  simpleMode = false,
  showInternals = false,
  askWhy = false,
}: {
  message: ChatMessage;
  index: number;
  compact?: boolean;
  simpleMode?: boolean;
  showInternals?: boolean;
  askWhy?: boolean;
}) {
  const isUser = message.role === "user";
  const [thoughtsExpanded, setThoughtsExpanded] = useState(false);
  const [innovationExpanded, setInnovationExpanded] = useState(false);
  const [copiedSectionId, setCopiedSectionId] = useState<string | null>(null);
  const [copiedMessage, setCopiedMessage] = useState(false);
  const [applyingPlan, setApplyingPlan] = useState(false);
  const [contentExpanded, setContentExpanded] = useState(!simpleMode);
  const vscode = useVSCodeAPI();
  const hasThoughts = message.thoughtChain && message.thoughtChain.length > 0;
  const hasExplainability = Boolean(message.explainability);
  const hasInnovation = Boolean(message.innovation);
  const pendingOperationsCount =
    message.fileOperations?.filter((operation) => operation.approved === undefined).length ?? 0;
  const hasSections = !isUser && (message.sections?.length ?? 0) > 0;
  const outcome = useMemo(() => deriveOutcome(message), [message]);
  const shouldShowExplainability = !isUser && hasExplainability && (showInternals || askWhy);
  const shouldShowInnovation = !isUser && hasInnovation && showInternals;

  useEffect(() => {
    setContentExpanded(!simpleMode);
  }, [simpleMode]);

  const copyText = async (text: string) => {
    if (typeof navigator !== "undefined" && navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(text);
      return;
    }
    const textarea = document.createElement("textarea");
    textarea.value = text;
    textarea.style.position = "fixed";
    textarea.style.opacity = "0";
    document.body.appendChild(textarea);
    textarea.select();
    document.execCommand("copy");
    document.body.removeChild(textarea);
  };

  const handleCopySection = async (sectionId: string, content: string) => {
    try {
      await copyText(content);
      setCopiedSectionId(sectionId);
      setTimeout(() => setCopiedSectionId((current) => (current === sectionId ? null : current)), 1200);
    } catch {
      setCopiedSectionId(null);
    }
  };

  const handleCopyMessage = async () => {
    try {
      await copyText(message.content);
      setCopiedMessage(true);
      setTimeout(() => setCopiedMessage(false), 1200);
    } catch {
      setCopiedMessage(false);
    }
  };

  const handleApplyPlan = () => {
    if (pendingOperationsCount === 0) return;
    setApplyingPlan(true);
    vscode.postMessage({
      type: "applySafeWritePlan",
      messageId: message.id,
    });
    setTimeout(() => setApplyingPlan(false), 1500);
  };

  return (
    <div className={cn(
      "flex w-full animate-in fade-in slide-in-from-bottom-3 duration-500 fill-mode-both",
      isUser ? "justify-end" : "justify-start"
    )} style={{ animationDelay: `${Math.min(index * 50, 300)}ms` }}>
      <div className={cn(
        compact ? "flex max-w-[92%] gap-2" : "flex max-w-[85%] gap-3",
        isUser ? "flex-row-reverse" : "flex-row"
      )}>
        {/* AVATAR */}
        <div className="shrink-0 mt-1">
          <div className={cn(
            "size-7 rounded-lg flex items-center justify-center text-[10px] font-bold shadow-sm border",
            isUser ? "bg-primary text-primary-foreground border-primary/20" : "bg-card text-foreground border-border"
          )}>
            {isUser ? 'U' : 'S'}
          </div>
        </div>

        {/* CONTENT */}
        <div className={cn("space-y-2 w-full", isUser ? "items-end" : "items-start")}>
          <div className={cn(
            compact
              ? "px-3 py-2 rounded-xl shadow-sm text-[12px] leading-relaxed border"
              : "px-4 py-3 rounded-2xl shadow-sm text-sm leading-relaxed border",
            isUser 
              ? "bg-primary/5 border-primary/10 text-foreground rounded-tr-none" 
              : "bg-card border-border text-foreground rounded-tl-none"
          )}>
            {!isUser && (
              <div className="text-[10px] font-bold uppercase tracking-widest text-primary mb-1.5 opacity-80 flex justify-between items-center gap-2">
                <span>Sentinel AI</span>
                <div className="flex items-center gap-2">
                  <Button
                    size="xs"
                    variant="outline"
                    onClick={handleCopyMessage}
                    className="h-6 text-[9px] normal-case"
                  >
                    {copiedMessage ? "Copied" : "Copy"}
                  </Button>
                  {hasThoughts && showInternals && (
                    <button 
                      onClick={() => setThoughtsExpanded(!thoughtsExpanded)}
                      className="text-[9px] hover:underline normal-case font-normal opacity-60"
                    >
                      {thoughtsExpanded ? "Hide thoughts" : "Show thoughts"}
                    </button>
                  )}
                  {hasExplainability && (showInternals || askWhy) && (
                    <span className="text-[9px] normal-case opacity-60">Explainable turn</span>
                  )}
                  {hasInnovation && showInternals && (
                    <button
                      onClick={() => setInnovationExpanded((prev) => !prev)}
                      className="text-[9px] hover:underline normal-case font-normal opacity-60"
                    >
                      {innovationExpanded ? "Hide innovation" : "Show innovation"}
                    </button>
                  )}
                </div>
              </div>
            )}

            {/* Thought Chain */}
            {!isUser && hasThoughts && thoughtsExpanded && showInternals && (
              <div className="mb-3 p-2 bg-muted/50 rounded-lg border border-border/50 font-mono text-[11px] text-muted-foreground animate-in fade-in zoom-in-95 duration-200">
                {message.thoughtChain!.map((thought, i) => (
                  <div key={i} className="mb-1 last:mb-0">• {thought}</div>
                ))}
              </div>
            )}

            {!isUser && simpleMode && (
              <div className="sentinel-outcome-card">
                <div>
                  <h5>What I changed</h5>
                  <ul>
                    {outcome.changed.map((item) => (
                      <li key={`changed-${item}`}>{item}</li>
                    ))}
                  </ul>
                </div>
                <div>
                  <h5>What needs your approval</h5>
                  <ul>
                    {outcome.approval.map((item) => (
                      <li key={`approval-${item}`}>{item}</li>
                    ))}
                  </ul>
                </div>
                <div>
                  <h5>What happens next</h5>
                  <ul>
                    {outcome.next.map((item) => (
                      <li key={`next-${item}`}>{item}</li>
                    ))}
                  </ul>
                </div>
              </div>
            )}

            {(!simpleMode || contentExpanded || message.streaming) && (
              <div 
                className="prose prose-sm dark:prose-invert max-w-none prose-p:leading-relaxed prose-pre:bg-muted prose-pre:border prose-pre:rounded-lg sentinel-selectable-content"
                dangerouslySetInnerHTML={{ __html: renderMarkdown(message.content) }} 
              />
            )}

            {!isUser && simpleMode && !message.streaming && (
              <div className="mt-2 flex justify-end">
                <Button
                  size="xs"
                  variant="outline"
                  onClick={() => setContentExpanded((value) => !value)}
                  className="h-6 text-[9px] normal-case"
                >
                  {contentExpanded ? "Hide full response" : "Show full response"}
                </Button>
              </div>
            )}

            {message.streaming && (
              <span className="inline-block w-1.5 h-4 ml-1 bg-primary/40 animate-pulse align-middle" />
            )}
          </div>

          {shouldShowExplainability && message.explainability && (
            <div className="text-[11px] border rounded-lg p-2 bg-card/60 space-y-1">
              <div className="font-semibold text-[10px] uppercase tracking-wider text-muted-foreground">Turn Explainability</div>
              {message.explainability.intent_summary && (
                <div><span className="text-muted-foreground">Intent:</span> {message.explainability.intent_summary}</div>
              )}
              {typeof message.explainability.alignment_score === "number" && (
                <div>
                  <span className="text-muted-foreground">Alignment:</span>{" "}
                  {message.explainability.alignment_score.toFixed(1)}%
                </div>
              )}
              {typeof message.explainability.reliability_healthy === "boolean" && (
                <div>
                  <span className="text-muted-foreground">Reliability:</span>{" "}
                  {message.explainability.reliability_healthy ? "Healthy" : "Violated"}
                </div>
              )}
              {message.explainability.context_provider && (
                <div>
                  <span className="text-muted-foreground">Context provider:</span>{" "}
                  {message.explainability.context_provider}
                </div>
              )}
              {message.explainability.context_policy_mode && (
                <div>
                  <span className="text-muted-foreground">Policy mode:</span>{" "}
                  {message.explainability.context_policy_mode}
                </div>
              )}
              {message.explainability.context_fallback_reason && (
                <div>
                  <span className="text-muted-foreground">Fallback:</span>{" "}
                  {message.explainability.context_fallback_reason}
                </div>
              )}
              {message.explainability.evidence?.length ? (
                <div className="text-muted-foreground">
                  Evidence: {message.explainability.evidence.join(" | ")}
                </div>
              ) : null}
            </div>
          )}

          {shouldShowInnovation && message.innovation && (
            <div className="text-[11px] border rounded-lg p-2 bg-card/60 space-y-1">
              <div className="font-semibold text-[10px] uppercase tracking-wider text-muted-foreground">
                Innovation Trace
              </div>
              <div>
                <span className="text-muted-foreground">Recommended plan:</span>{" "}
                {message.innovation.counterfactual_plans?.recommended_plan_id ?? "n/a"}
              </div>
              <div>
                <span className="text-muted-foreground">Policy simulation:</span>{" "}
                {message.innovation.policy_simulation?.available === false
                  ? "unavailable"
                  : "available"}
              </div>
              <div>
                <span className="text-muted-foreground">Team graph:</span>{" "}
                {message.innovation.team_memory_graph?.node_count ?? 0} nodes /{" "}
                {message.innovation.team_memory_graph?.edge_count ?? 0} edges
              </div>
              {innovationExpanded && (
                <>
                  {message.innovation.constitutional_spec?.constraints?.length ? (
                    <div className="text-muted-foreground">
                      Constraints: {message.innovation.constitutional_spec.constraints.join(" | ")}
                    </div>
                  ) : null}
                  {message.innovation.policy_simulation?.modes?.length ? (
                    <div className="text-muted-foreground">
                      Modes:{" "}
                      {message.innovation.policy_simulation.modes
                        .map((mode) =>
                          `${mode.mode ?? "unknown"}=${mode.healthy ? "ok" : "violated"}`,
                        )
                        .join(" | ")}
                    </div>
                  ) : null}
                  {message.innovation.replay_ledger?.entry?.turn_id ? (
                    <div className="text-muted-foreground">
                      Replay turn: {message.innovation.replay_ledger.entry.turn_id.slice(0, 12)}
                    </div>
                  ) : null}
                </>
              )}
            </div>
          )}

          {hasSections && showInternals && (
            <div className="rounded-lg border bg-card/60 p-2 space-y-1.5">
              <div className="text-[10px] uppercase tracking-wider text-muted-foreground font-semibold">
                Implementation Sections
              </div>
              {message.sections!.map((section) => (
                <div
                  key={section.id}
                  className="flex items-center justify-between gap-2 rounded-md border border-border/70 px-2 py-1.5"
                >
                  <div className="min-w-0">
                    <div className="text-[11px] font-medium truncate">{section.title}</div>
                    {section.pathHint ? (
                      <div className="text-[10px] text-muted-foreground truncate">{section.pathHint}</div>
                    ) : null}
                  </div>
                  <Button
                    size="xs"
                    variant="outline"
                    onClick={() => handleCopySection(section.id, section.content)}
                    className="h-6"
                  >
                    {copiedSectionId === section.id ? "Copied" : "Copy section"}
                  </Button>
                </div>
              ))}
            </div>
          )}

          {/* TOOL CALLS & ACTIONS */}
          {((showInternals && message.toolCalls) || message.fileOperations) && (
            <div className="space-y-2 mt-2 w-full">
              {!isUser && pendingOperationsCount > 0 && (
                <div className="flex justify-end">
                  <Button
                    size="sm"
                    onClick={handleApplyPlan}
                    disabled={applyingPlan}
                    className="h-8"
                  >
                    {applyingPlan
                      ? "Applying..."
                      : `Apply safe_write plan (${pendingOperationsCount})`}
                  </Button>
                </div>
              )}
              {showInternals && message.toolCalls?.map((tool, i) => (
                <div key={`tool-${i}`} className="animate-in fade-in slide-in-from-left-2 duration-300 fill-mode-both" style={{ animationDelay: '150ms' }}>
                   <ToolCallCard tool={tool} />
                </div>
              ))}
              {message.fileOperations?.map((op, i) => (
                <div key={`op-${i}`} className="animate-in fade-in slide-in-from-left-2 duration-300 fill-mode-both" style={{ animationDelay: '200ms' }}>
                  <FileApproval operation={op} messageId={message.id} />
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
