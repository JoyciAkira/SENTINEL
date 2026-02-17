import React from "react";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";
import {
  Sparkles,
  Terminal,
  Shield,
  Target,
  Brain,
  HelpCircle,
  PlayCircle,
  Wand2,
  DraftingCompass,
  LayoutTemplate,
  CalendarClock,
  ChevronDown,
  ChevronUp,
} from "lucide-react";
import { cn } from "@/lib/utils";

const STARTER_PROMPTS = [
  {
    label: "Client Portal",
    text: "Build a clean client portal where customers can sign in, track requests, and chat with support. Keep it production-ready and simple.",
    icon: LayoutTemplate,
  },
  {
    label: "Booking App",
    text: "Create a booking app for a small business with calendar scheduling, reminder emails, and role-based access.",
    icon: CalendarClock,
  },
  {
    label: "Spreadsheet to App",
    text: "Turn this spreadsheet workflow into a web app with forms, approval steps, and an admin dashboard.",
    icon: Sparkles,
  },
];

const ADVANCED_ACTIONS = [
  { label: "Execute First Pending", text: "/execute-first-pending", icon: PlayCircle },
  {
    label: "Orchestrate",
    text: "/orchestrate Harden auth + payments flow --parallel=2 --count=4 --modes=plan,build,review",
    icon: DraftingCompass,
  },
  { label: "Memory Status", text: "/memory-status", icon: Brain },
  { label: "Refine AppSpec", text: "/appspec-refine", icon: Wand2 },
  { label: "AppSpec Plan", text: "/appspec-plan", icon: DraftingCompass },
  { label: "Help", text: "/help", icon: HelpCircle },
];

type PromptItem = {
  label: string;
  text: string;
  icon: React.ComponentType<{ className?: string }>;
};

export default function QuickPrompts({
  goalsCount,
  pendingApprovals,
  alignmentScore,
  hasConversation,
}: {
  goalsCount?: number;
  pendingApprovals?: number;
  alignmentScore?: number;
  hasConversation?: boolean;
}) {
  const [showAdvanced, setShowAdvanced] = React.useState(false);
  const connected = useStore((s) => s.connected);
  const addMessage = useStore((s) => s.addMessage);
  const goals = useStore((s) => s.goals);
  const alignment = useStore((s) => s.alignment);
  const messages = useStore((s) => s.messages);
  const vscode = useVSCodeAPI();
  const resolvedGoalsCount = goalsCount ?? goals.length;
  const resolvedPendingApprovals =
    pendingApprovals ??
    messages.reduce(
      (acc, msg) => acc + (msg.fileOperations?.filter((op) => op.approved !== true).length ?? 0),
      0,
    );
  const resolvedAlignmentScore = alignmentScore ?? alignment?.score ?? 0;
  const resolvedHasConversation = hasConversation ?? messages.length > 0;

  const contextualStarters = React.useMemo<PromptItem[]>(() => {
    if (resolvedGoalsCount === 0) {
      return STARTER_PROMPTS;
    }
    if (resolvedPendingApprovals > 0) {
      return [
        {
          label: "Resolve Approvals",
          text: "Summarize pending approvals and propose the safest next action.",
          icon: Shield,
        },
        ...STARTER_PROMPTS.slice(0, 2),
      ];
    }
    if (!resolvedHasConversation) {
      return STARTER_PROMPTS;
    }
    return [
      {
        label: "Next Milestone",
        text: "Plan the next milestone from current goals with minimal risk.",
        icon: Target,
      },
      ...STARTER_PROMPTS.slice(0, 2),
    ];
  }, [resolvedGoalsCount, resolvedPendingApprovals, resolvedHasConversation]);

  const contextualCoreActions = React.useMemo<PromptItem[]>(() => {
    const base: PromptItem[] = [];
    if (resolvedGoalsCount === 0) {
      base.push({ label: "Initialize", text: "/init", icon: Sparkles });
    } else {
      base.push({ label: "List Goals", text: "What are the active goals?", icon: Target });
      base.push({ label: "Execute First Pending", text: "/execute-first-pending", icon: PlayCircle });
    }
    if (resolvedAlignmentScore < 80) {
      base.push({ label: "Improve Alignment", text: "How can we improve alignment before next step?", icon: Shield });
    } else {
      base.push({ label: "Verify Action", text: "Validate my next changes", icon: Terminal });
    }
    if (resolvedPendingApprovals > 0) {
      base.push({ label: "Approval Status", text: "/memory-status", icon: Brain });
    }
    return base.slice(0, 5);
  }, [resolvedGoalsCount, resolvedAlignmentScore, resolvedPendingApprovals]);

  const handleSend = (text: string) => {
    if (!connected) return;

    addMessage({
      id: crypto.randomUUID(),
      role: "user",
      content: text,
      timestamp: Date.now(),
    });

    vscode.postMessage({
      type: "chatMessage",
      text,
    });
  };

  return (
    <div className="sentinel-quick-prompts animate-in fade-in slide-in-from-bottom-1 duration-500 delay-100 fill-mode-both">
      <div className="sentinel-quick-prompts__starters">
        {contextualStarters.map((prompt) => (
          <button
            key={prompt.label}
            type="button"
            disabled={!connected}
            onClick={() => handleSend(prompt.text)}
            className={cn(
              "sentinel-quick-prompts__starter",
              "disabled:opacity-50 disabled:cursor-not-allowed"
            )}
          >
            <prompt.icon className="size-3.5" />
            <span>{prompt.label}</span>
          </button>
        ))}
      </div>

      <div className="sentinel-quick-prompts__actions">
        {contextualCoreActions.map((prompt) => (
          <button
            key={prompt.label}
            type="button"
            disabled={!connected}
            onClick={() => handleSend(prompt.text)}
            className={cn(
              "sentinel-quick-prompts__pill",
              "disabled:opacity-50 disabled:cursor-not-allowed"
            )}
          >
            <prompt.icon className="size-3" />
            {prompt.label}
          </button>
        ))}

        <button
          type="button"
          disabled={!connected}
          className="sentinel-quick-prompts__more"
          onClick={() => setShowAdvanced((prev) => !prev)}
        >
          {showAdvanced ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
          {showAdvanced ? "Hide advanced" : "More actions"}
        </button>
      </div>

      {showAdvanced && (
        <div className="sentinel-quick-prompts__advanced">
          {ADVANCED_ACTIONS.map((prompt) => (
            <button
              key={prompt.label}
              type="button"
              disabled={!connected}
              onClick={() => handleSend(prompt.text)}
              className={cn(
                "sentinel-quick-prompts__pill sentinel-quick-prompts__pill--muted",
                "disabled:opacity-50 disabled:cursor-not-allowed"
              )}
            >
              <prompt.icon className="size-3" />
              {prompt.label}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
