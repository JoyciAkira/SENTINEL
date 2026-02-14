import assert from "node:assert/strict";
import {
  decideIntentRoute,
  inferOrchestrationCommandFromIntent,
} from "../src/webview-providers/intent-routing";
import {
  BLUEPRINTS,
  buildBlueprintPrompt,
  detectBlueprintFromIntent,
  getBlueprintById,
} from "../src/webview-providers/blueprints";

function runIntentRoutingTests(): void {
  const orchestrateDecision = decideIntentRoute({
    trimmedText:
      "Implement end-to-end hardening for auth, payments, rollout safety and regression checks in phased steps.",
    hasAppSpecDraft: false,
    supportsOrchestration: true,
    supportsGoalExecution: true,
  });
  assert.equal(orchestrateDecision.kind, "orchestrate");
  if (orchestrateDecision.kind === "orchestrate") {
    assert.ok(orchestrateDecision.command.maxParallel >= 2);
    assert.ok(orchestrateDecision.command.subtaskCount >= 4);
    assert.ok(orchestrateDecision.command.modes?.includes("review"));
  }

  const executePendingDecision = decideIntentRoute({
    trimmedText: "please execute first pending goal now",
    hasAppSpecDraft: false,
    supportsOrchestration: true,
    supportsGoalExecution: true,
  });
  assert.equal(executePendingDecision.kind, "execute_first_pending");

  const appSpecRefineDecision = decideIntentRoute({
    trimmedText: "refine appspec for stronger data model constraints",
    hasAppSpecDraft: true,
    supportsOrchestration: true,
    supportsGoalExecution: true,
  });
  assert.equal(appSpecRefineDecision.kind, "appspec_refine");

  const slashIsIgnored = decideIntentRoute({
    trimmedText: "/orchestrate ship this",
    hasAppSpecDraft: true,
    supportsOrchestration: true,
    supportsGoalExecution: true,
  });
  assert.equal(slashIsIgnored.kind, "none");

  const inferred = inferOrchestrationCommandFromIntent(
    "Plan, implement, review, and deploy a reliability migration with performance and compliance checks.",
  );
  assert.ok(inferred.maxParallel >= 2 && inferred.maxParallel <= 4);
  assert.ok(inferred.subtaskCount >= 4 && inferred.subtaskCount <= 6);
}

function runBlueprintTests(): void {
  assert.ok(BLUEPRINTS.length >= 4, "Blueprint catalog should contain at least 4 entries");

  const booking = detectBlueprintFromIntent(
    "We need a booking and reservation app with calendar scheduling.",
  );
  assert.ok(booking, "Booking intent should match a blueprint");
  assert.equal(booking?.id, "booking-suite");

  const crm = getBlueprintById("crm-lite");
  assert.ok(crm, "CRM blueprint should exist");
  const prompt = buildBlueprintPrompt(crm!, "Track deals and forecast revenue");
  assert.ok(prompt.includes("Track deals and forecast revenue"));
  assert.ok(prompt.includes("Non-negotiable guardrails"));
}

function main(): void {
  runIntentRoutingTests();
  runBlueprintTests();
  process.stdout.write("intent-router + blueprint tests passed\n");
}

main();
