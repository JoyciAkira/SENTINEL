export const ORCHESTRATION_ALLOWED_MODES = new Set(["plan", "build", "review", "deploy"]);

export interface ParsedOrchestrationCommand {
  task: string;
  maxParallel: number;
  subtaskCount: number;
  modes?: string[];
}

export type IntentRouteDecision =
  | { kind: "none" }
  | { kind: "execute_first_pending"; reason: string }
  | { kind: "appspec_refine"; reason: string }
  | { kind: "appspec_plan"; reason: string }
  | { kind: "orchestrate"; reason: string; command: ParsedOrchestrationCommand };

export interface IntentRouteInput {
  trimmedText: string;
  hasAppSpecDraft: boolean;
  supportsOrchestration: boolean;
  supportsGoalExecution: boolean;
}

function regexCount(source: string, pattern: RegExp): number {
  return source.match(pattern)?.length ?? 0;
}

export function inferOrchestrationCommandFromIntent(text: string): ParsedOrchestrationCommand {
  const lower = text.toLowerCase();
  const complexityScore =
    (text.length > 120 ? 1 : 0) +
    (text.length > 220 ? 1 : 0) +
    (/\b(end-to-end|end to end|e2e)\b/.test(lower) ? 1 : 0) +
    (/\b(security|performance|scalability|reliability|migration|compliance)\b/.test(lower)
      ? 1
      : 0) +
    (regexCount(lower, /,|;|\band\b|\be\b/g) >= 3 ? 1 : 0);

  const includeDeploy = /\b(deploy|release|rollout|production|prod)\b/.test(lower);
  const maxParallel = Math.max(1, Math.min(4, complexityScore >= 3 ? 3 : 2));
  const subtaskCount = Math.max(2, Math.min(6, includeDeploy ? 5 : complexityScore >= 3 ? 5 : 4));
  const modes = includeDeploy
    ? ["plan", "build", "review", "deploy"]
    : ["plan", "build", "review"];

  return {
    task: text.trim(),
    maxParallel,
    subtaskCount,
    modes,
  };
}

export function decideIntentRoute(input: IntentRouteInput): IntentRouteDecision {
  const { trimmedText, hasAppSpecDraft, supportsOrchestration, supportsGoalExecution } = input;
  if (!trimmedText || trimmedText.startsWith("/")) {
    return { kind: "none" };
  }

  const lower = trimmedText.toLowerCase();
  const asksExecutePending =
    /\b(execute first pending|first pending goal|next pending goal|primo goal pending)\b/.test(
      lower,
    );
  if (asksExecutePending && supportsGoalExecution) {
    return {
      kind: "execute_first_pending",
      reason: "Detected intent to execute the next pending goal.",
    };
  }

  const mentionsAppSpec = /\b(appspec|app spec)\b/.test(lower);
  const asksRefine = /\b(refine|improve|migliora|tighten|harden|revise)\b/.test(lower);
  const asksPlan = /\b(plan|planning|roadmap|steps|phases|decompose|scomponi)\b/.test(lower);
  if (mentionsAppSpec && asksRefine && hasAppSpecDraft) {
    return {
      kind: "appspec_refine",
      reason: "Detected AppSpec refinement request with available draft.",
    };
  }
  if (mentionsAppSpec && asksPlan && hasAppSpecDraft) {
    return {
      kind: "appspec_plan",
      reason: "Detected AppSpec planning request with available draft.",
    };
  }

  const explicitOrchestration =
    /\b(orchestrate|orchestration|orchestrator|orchestrazione|decompose|scomponi|break down)\b/.test(
      lower,
    );
  const actionVerb =
    /\b(build|implement|create|design|refactor|optimi[sz]e|ship|deliver|harden|migrate|upgrade|setup|add|fix|improve|sviluppa|implementa|crea|ottimizza|migliora)\b/.test(
      lower,
    );
  const planningSignal =
    /\b(roadmap|subtasks?|step-by-step|step by step|phases?|milestones?|piano|fasi)\b/.test(lower);
  const complexitySignal =
    trimmedText.length > 160 ||
    regexCount(lower, /,|;|\band\b|\be\b/g) >= 3 ||
    /\b(end-to-end|end to end|e2e)\b/.test(lower);

  const shouldOrchestrate =
    explicitOrchestration || (actionVerb && planningSignal) || (actionVerb && complexitySignal);
  if (shouldOrchestrate && supportsOrchestration) {
    return {
      kind: "orchestrate",
      reason: "Detected complex implementation intent; auto-routing to bounded orchestration.",
      command: inferOrchestrationCommandFromIntent(trimmedText),
    };
  }

  return { kind: "none" };
}
