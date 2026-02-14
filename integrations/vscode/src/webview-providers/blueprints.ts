export interface BlueprintDefinition {
  id: string;
  title: string;
  audience: "no-code" | "developer" | "mixed";
  summary: string;
  modules: string[];
  guardrails: string[];
  keywords: string[];
}

export const BLUEPRINTS: BlueprintDefinition[] = [
  {
    id: "client-portal",
    title: "Client Portal",
    audience: "mixed",
    summary: "Customer sign-in, request tracking, messaging, and admin operations.",
    modules: [
      "Auth and roles",
      "Request lifecycle board",
      "Customer messaging thread",
      "Admin analytics dashboard",
    ],
    guardrails: [
      "Contract-preserving APIs",
      "Approval gate on sensitive writes",
      "Audit trail for user actions",
    ],
    keywords: ["client portal", "customer portal", "support portal", "requests", "ticketing"],
  },
  {
    id: "booking-suite",
    title: "Booking Suite",
    audience: "no-code",
    summary: "Scheduling, reminders, availability calendar, and role-based access.",
    modules: [
      "Calendar and slots",
      "Booking form and reschedule",
      "Reminder notifications",
      "Provider/admin panel",
    ],
    guardrails: [
      "Idempotent booking writes",
      "Timezone-safe date handling",
      "Deterministic conflict resolution",
    ],
    keywords: ["booking", "reservation", "calendar", "appointment", "schedule"],
  },
  {
    id: "ops-dashboard",
    title: "Ops Dashboard",
    audience: "developer",
    summary: "Operational monitoring, incidents, approvals, and reliability controls.",
    modules: [
      "KPI monitoring board",
      "Incident workflow",
      "Approval queue",
      "Reliability and policy center",
    ],
    guardrails: [
      "Read/write separation",
      "Policy checks before action",
      "SLO-based rollback triggers",
    ],
    keywords: ["ops", "operations", "dashboard", "incident", "monitoring", "kpi"],
  },
  {
    id: "crm-lite",
    title: "CRM Lite",
    audience: "mixed",
    summary: "Leads, pipeline, customer notes, and sales activity tracking.",
    modules: [
      "Lead and account records",
      "Pipeline stages",
      "Activity timeline",
      "Revenue dashboard",
    ],
    guardrails: [
      "Strict data validation",
      "Role-based entity permissions",
      "Deterministic import/export mappings",
    ],
    keywords: ["crm", "sales", "pipeline", "lead", "customer management"],
  },
];

export function listBlueprintsForChat(): string {
  return BLUEPRINTS.map(
    (blueprint) =>
      `- \`${blueprint.id}\` · ${blueprint.title}: ${blueprint.summary}`,
  ).join("\n");
}

export function getBlueprintById(id: string): BlueprintDefinition | undefined {
  const normalized = id.trim().toLowerCase();
  return BLUEPRINTS.find((blueprint) => blueprint.id === normalized);
}

export function detectBlueprintFromIntent(text: string): BlueprintDefinition | null {
  const lower = text.toLowerCase();
  const matched = BLUEPRINTS.find((blueprint) =>
    blueprint.keywords.some((keyword) => lower.includes(keyword)),
  );
  return matched ?? null;
}

export function buildBlueprintPrompt(
  blueprint: BlueprintDefinition,
  customization?: string,
): string {
  const custom = customization?.trim();
  const customGoal = custom && custom.length > 0 ? custom : blueprint.summary;
  const modules = blueprint.modules.map((module) => `- ${module}`).join("\n");
  const guardrails = blueprint.guardrails.map((rule) => `- ${rule}`).join("\n");

  return (
    `Build this application using the "${blueprint.title}" blueprint.\n` +
    `Primary goal: ${customGoal}\n` +
    `Audience: ${blueprint.audience}\n\n` +
    `Required modules:\n${modules}\n\n` +
    `Non-negotiable guardrails:\n${guardrails}\n\n` +
    "Output policy:\n" +
    "- outcome-first summary\n" +
    "- bounded and deterministic implementation plan\n" +
    "- explicit approvals before sensitive writes\n" +
    "- minimal, reversible file changes\n"
  );
}
