import React, { Suspense, lazy, useEffect, useMemo, useRef, useState } from "react";
import { Goal } from "@sentinel/sdk";
import { useStore } from "./state/store";
import { useVSCodeAPI } from "./hooks/useVSCodeAPI";
import { useMCPMessages } from "./hooks/useMCPMessages";
import {
  Activity,
  CheckCircle2,
  Gauge,
  History,
  LayoutDashboard,
  MessageSquare,
  Orbit,
  Settings,
  ShieldAlert,
  ShieldCheck,
  Target,
  Wrench,
  Play,
  Pause,
  RotateCcw,
  PanelRightOpen,
  LayoutPanelTop,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { Button } from "./components/ui/button";
import { Badge } from "./components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { ScrollArea } from "./components/ui/scroll-area";

import MessageList from "./components/Chat/MessageList";
import ChatInput from "./components/Chat/ChatInput";
import QuickPrompts from "./components/Chat/QuickPrompts";
const GoalTree = lazy(() => import("./components/Goals/GoalTree"));
const GoalGraph = lazy(() => import("./components/AtomicForge/GoalGraph"));
const AppSpecPreview = lazy(() => import("./components/AppSpec/AppSpecPreview"));
const PinnedTranscript = lazy(() => import("./components/Memory/PinnedTranscript"));
const QualityDashboard = lazy(() => import("./components/Quality/QualityDashboard"));

type PageId = "command" | "chat" | "forge" | "network" | "audit" | "settings";
type TimelineStage = "all" | "received" | "plan" | "tool" | "stream" | "approval" | "result" | "error" | "cancel";
type ThemePreset = "mono-mint" | "warm-graphite" | "pure-vscode";
type UiMode = "simple" | "advanced";
type GuidedStage = "goal" | "plan" | "apply";

const RISK_LEVELS = [
  { label: "Low", min: 85, className: "sentinel-risk-low" },
  { label: "Moderate", min: 65, className: "sentinel-risk-moderate" },
  { label: "High", min: 0, className: "sentinel-risk-high" },
] as const;

function statusToProgress(status: string): number {
  switch (status.toLowerCase()) {
    case "completed":
      return 100;
    case "inprogress":
    case "active":
      return 55;
    case "blocked":
      return 15;
    default:
      return 0;
  }
}

function shortFingerprint(id: string, timestamp: number): string {
  const compactId = id.replace(/-/g, "").slice(0, 8).toUpperCase();
  const compactTs = (timestamp % 100000).toString().padStart(5, "0");
  return `${compactId}-${compactTs}`;
}

function median(values: number[]): number {
  if (values.length === 0) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 0) {
    return (sorted[middle - 1] + sorted[middle]) / 2;
  }
  return sorted[middle];
}

export default function App() {
  const vscodeApi = useVSCodeAPI();
  useMCPMessages(vscodeApi);

  const connected = useStore((s) => s.connected);
  const alignment = useStore((s) => s.alignment);
  const goals = useStore((s) => s.goals);
  const messages = useStore((s) => s.messages);
  const reliability = useStore((s) => s.reliability);
  const reliabilityThresholds = useStore((s) => s.reliabilityThresholds);
  const reliabilitySlo = useStore((s) => s.reliabilitySlo);
  const governance = useStore((s) => s.governance);
  const policyAction = useStore((s) => s.policyAction);
  const timeline = useStore((s) => s.timeline);
  const clearTimeline = useStore((s) => s.clearTimeline);
  const runtimeCapabilities = useStore((s) => s.runtimeCapabilities);
  const augmentSettings = useStore((s) => s.augmentSettings);
  const qualityStatus = useStore((s) => s.qualityStatus);
  const uiKpiHistory = useStore((s) => s.uiKpiHistory);
  const addMessage = useStore((s) => s.addMessage);

  const [activePage, setActivePage] = useState<PageId>("chat");
  const [showPinnedTranscript, setShowPinnedTranscript] = useState(false);
  const [showQualityDashboard, setShowQualityDashboard] = useState(false);
  const [selectedGoal, setSelectedGoal] = useState<Goal | null>(null);
  const [rejectReason, setRejectReason] = useState("Policy conflict with current project direction");
  const [lockRequiredSeed, setLockRequiredSeed] = useState(true);
  const [timelineReplay, setTimelineReplay] = useState(false);
  const [timelineCursor, setTimelineCursor] = useState(0);
  const [timelineStageFilter, setTimelineStageFilter] = useState<TimelineStage>("all");
  const [timelineTurnFilter, setTimelineTurnFilter] = useState("");
  const [uiMode, setUiMode] = useState<UiMode>("simple");
  const [showChatDetails, setShowChatDetails] = useState(false);
  const [askWhy, setAskWhy] = useState(false);
  const [showPreviewPanel, setShowPreviewPanel] = useState(true);
  const [themePreset, setThemePreset] = useState<ThemePreset>("mono-mint");
  const [compactDensity, setCompactDensity] = useState(false);
  const [chatMessagesHeight, setChatMessagesHeight] = useState(620);
  const [timelineWidth, setTimelineWidth] = useState(320);
  const [onboardingOpen, setOnboardingOpen] = useState(false);
  const [onboardingStep, setOnboardingStep] = useState(1);
  const [onboardingMode, setOnboardingMode] = useState<"no-code" | "developer">("no-code");
  const [onboardingGoal, setOnboardingGoal] = useState("");
  const [onboardingUsers, setOnboardingUsers] = useState("");
  const [onboardingPriority, setOnboardingPriority] = useState<"speed" | "quality" | "compliance">("quality");
  const chatHeightRef = useRef(chatMessagesHeight);
  const timelineWidthRef = useRef(timelineWidth);
  const simpleMode = uiMode === "simple";

  useEffect(() => {
    vscodeApi.postMessage({ type: "webviewReady" });
    vscodeApi.postMessage({ type: "refreshRuntimePolicies" });
  }, [vscodeApi]);

  useEffect(() => {
    const savedMode = window.localStorage.getItem("sentinel.ui.mode");
    const savedTheme = window.localStorage.getItem("sentinel.ui.theme");
    const savedDensity = window.localStorage.getItem("sentinel.ui.compact");
    if (savedMode === "simple" || savedMode === "advanced") {
      setUiMode(savedMode);
    }
    if (
      savedTheme === "mono-mint" ||
      savedTheme === "warm-graphite" ||
      savedTheme === "pure-vscode"
    ) {
      setThemePreset(savedTheme);
    }
    if (savedDensity === "true") {
      setCompactDensity(true);
    }
  }, []);

  useEffect(() => {
    window.localStorage.setItem("sentinel.ui.mode", uiMode);
  }, [uiMode]);

  useEffect(() => {
    window.localStorage.setItem("sentinel.ui.theme", themePreset);
  }, [themePreset]);

  useEffect(() => {
    window.localStorage.setItem("sentinel.ui.compact", compactDensity ? "true" : "false");
  }, [compactDensity]);

  useEffect(() => {
    const savedChatHeight = Number(window.localStorage.getItem("sentinel.ui.chatHeight") ?? "0");
    const savedTimelineWidth = Number(window.localStorage.getItem("sentinel.ui.timelineWidth") ?? "0");
    if (Number.isFinite(savedChatHeight) && savedChatHeight >= 280) {
      setChatMessagesHeight(savedChatHeight);
      chatHeightRef.current = savedChatHeight;
    }
    if (Number.isFinite(savedTimelineWidth) && savedTimelineWidth >= 220) {
      setTimelineWidth(savedTimelineWidth);
      timelineWidthRef.current = savedTimelineWidth;
    }
  }, []);

  useEffect(() => {
    window.localStorage.setItem("sentinel.ui.chatHeight", String(chatMessagesHeight));
    chatHeightRef.current = chatMessagesHeight;
  }, [chatMessagesHeight]);

  useEffect(() => {
    window.localStorage.setItem("sentinel.ui.timelineWidth", String(timelineWidth));
    timelineWidthRef.current = timelineWidth;
  }, [timelineWidth]);

  useEffect(() => {
    if (uiMode === "simple" && activePage !== "chat") {
      setActivePage("chat");
    }
    if (uiMode === "simple") {
      setShowChatDetails(false);
    }
  }, [uiMode, activePage]);

  useEffect(() => {
    const dismissed = window.localStorage.getItem("sentinel.onboarding.dismissed") === "true";
    if (!dismissed && simpleMode && messages.length === 0 && goals.length === 0) {
      setOnboardingOpen(true);
      return;
    }
    if (!simpleMode) {
      setOnboardingOpen(false);
    }
  }, [simpleMode, messages.length, goals.length]);

  const filteredTimeline = useMemo(() => {
    const turnPrefix = timelineTurnFilter.trim().toLowerCase();
    return timeline.filter((event) => {
      const stageOk = timelineStageFilter === "all" || event.stage === timelineStageFilter;
      const turnOk =
        turnPrefix.length === 0 ||
        (event.turnId?.toLowerCase().includes(turnPrefix) ?? false) ||
        event.id.toLowerCase().includes(turnPrefix);
      return stageOk && turnOk;
    });
  }, [timeline, timelineStageFilter, timelineTurnFilter]);

  useEffect(() => {
    if (!timelineReplay || filteredTimeline.length === 0) return;
    const timer = setInterval(() => {
      setTimelineCursor((prev) => {
        if (prev >= filteredTimeline.length - 1) {
          return 0;
        }
        return prev + 1;
      });
    }, 850);
    return () => clearInterval(timer);
  }, [timelineReplay, filteredTimeline.length]);

  useEffect(() => {
    if (timelineCursor >= filteredTimeline.length) {
      setTimelineCursor(Math.max(0, filteredTimeline.length - 1));
    }
  }, [timelineCursor, filteredTimeline.length]);

  const navItems = [
    { id: "command", label: "Command Center", icon: LayoutDashboard },
    { id: "chat", label: "Agent Chat", icon: MessageSquare },
    { id: "forge", label: "Goal Forge", icon: Target },
    { id: "transcript", label: "Pinned Transcript", icon: History },
    { id: "quality", label: "Quality Dashboard", icon: ShieldCheck },
    { id: "network", label: "Federation", icon: Orbit },
    { id: "audit", label: "Execution Log", icon: History },
    { id: "settings", label: "Runtime", icon: Settings },
  ] as const;

  const alignmentScore = alignment?.score ?? 0;
  const alignmentConfidence = ((alignment?.confidence ?? 0) * 100).toFixed(0);
  const alignmentStatus = alignment?.status ?? "Initializing";

  const completedGoals = useMemo(
    () => goals.filter((goal) => goal.status.toLowerCase() === "completed").length,
    [goals],
  );
  const pendingGoals = useMemo(
    () => goals.filter((goal) => goal.status.toLowerCase() === "pending"),
    [goals],
  );

  const toolCallsCount = useMemo(
    () => messages.reduce((acc, msg) => acc + (msg.toolCalls?.length ?? 0), 0),
    [messages],
  );

  const pendingFileApprovals = useMemo(
    () =>
      messages.reduce(
        (acc, msg) =>
          acc + (msg.fileOperations?.filter((operation) => operation.approved !== true).length ?? 0),
        0,
      ),
    [messages],
  );

  const risk = useMemo(() => {
    const level = RISK_LEVELS.find((candidate) => alignmentScore >= candidate.min) ?? RISK_LEVELS[2];
    return {
      label: level.label,
      className: level.className,
      drift: Math.max(0, 100 - alignmentScore).toFixed(1),
    };
  }, [alignmentScore]);

  const pageTitle = useMemo(
    () => navItems.find((item) => item.id === activePage)?.label ?? "Command Center",
    [activePage],
  );
  const lastUserMessage = useMemo(
    () => [...messages].reverse().find((message) => message.role === "user"),
    [messages],
  );
  const activeStreamingMessage = useMemo(
    () => [...messages].reverse().find((message) => message.role === "assistant" && message.streaming),
    [messages],
  );
  const latestAppSpec = useMemo(
    () => [...messages].reverse().find((message) => message.role === "assistant" && message.appSpec)?.appSpec ?? null,
    [messages],
  );
  const lastAssistantMessage = useMemo(
    () => [...messages].reverse().find((message) => message.role === "assistant") ?? null,
    [messages],
  );

  useEffect(() => {
    if (!latestAppSpec) {
      setShowPreviewPanel(false);
      return;
    }
    if (simpleMode) {
      setShowPreviewPanel(true);
    }
  }, [latestAppSpec, simpleMode]);

  const requestRuntimeRefresh = () => {
    vscodeApi.postMessage({ type: "refreshRuntimePolicies" });
  };

  const applyAugmentSettings = (next: typeof augmentSettings) => {
    vscodeApi.postMessage({
      type: "setAugmentSettings",
      settings: next,
    });
  };

  const sendChatPrompt = (text: string) => {
    if (!connected || Boolean(activeStreamingMessage)) return;
    addMessage({
      id: crypto.randomUUID(),
      role: "user",
      content: text,
      timestamp: Date.now(),
    });
    vscodeApi.postMessage({
      type: "chatMessage",
      text,
    });
  };

  const sendSlashCommand = (command: string) => {
    sendChatPrompt(command);
  };

  const currentTimelineEvent =
    filteredTimeline.length > 0 ? filteredTimeline[timelineCursor] : undefined;
  const worldModel = governance?.world_model;
  const showInternals = !simpleMode || showChatDetails;
  const showSimplePreview = simpleMode && !showChatDetails && showPreviewPanel && Boolean(latestAppSpec);
  const showTimelinePanel = !simpleMode || showChatDetails;
  const hasSidePanel = showSimplePreview || showTimelinePanel;
  const guidedStage = useMemo<GuidedStage>(() => {
    if (pendingFileApprovals > 0) return "apply";
    if (activeStreamingMessage) return "plan";
    if (messages.length === 0) return "goal";
    const hasPlanSignal =
      Boolean(latestAppSpec) ||
      Boolean(lastAssistantMessage?.content && /Orchestration ID:/i.test(lastAssistantMessage.content)) ||
      pendingGoals.length > 0;
    return hasPlanSignal ? "plan" : "goal";
  }, [
    pendingFileApprovals,
    activeStreamingMessage,
    messages.length,
    latestAppSpec,
    lastAssistantMessage,
    pendingGoals.length,
  ]);
  const guidedHint = useMemo(() => {
    if (guidedStage === "goal") {
      return "Describe the end result. Sentinel chooses the safest execution path automatically.";
    }
    if (guidedStage === "plan") {
      return "Plan ready. Review the summarized outcome and ask for refinements if needed.";
    }
    return "Apply stage. Approve pending file operations to execute changes safely.";
  }, [guidedStage]);
  const uiKpi = useMemo(() => {
    const userTurns = messages.filter((message) => message.role === "user");
    const assistantTurns = messages.filter((message) => message.role === "assistant");
    const slashTurns = userTurns.filter((message) => message.content.trim().startsWith("/")).length;
    const autoRoutedTurns = assistantTurns.filter((message) =>
      message.content.startsWith("Auto-routed by Intent Router."),
    ).length;

    const planLatency: number[] = [];
    const receivedByTurn = new Map<string, number>();
    const firstPlanByTurn = new Map<string, number>();
    for (const event of timeline) {
      if (!event.turnId) continue;
      if (event.stage === "received" && !receivedByTurn.has(event.turnId)) {
        receivedByTurn.set(event.turnId, event.timestamp);
      }
      if (event.stage === "plan" && !firstPlanByTurn.has(event.turnId)) {
        firstPlanByTurn.set(event.turnId, event.timestamp);
      }
    }
    for (const [turnId, receivedTs] of receivedByTurn.entries()) {
      const planTs = firstPlanByTurn.get(turnId);
      if (planTs && planTs >= receivedTs) {
        planLatency.push(planTs - receivedTs);
      }
    }

    const fileOps = assistantTurns.flatMap((message) => message.fileOperations ?? []);
    const resolvedApprovals = fileOps.filter((operation) => operation.approved !== undefined);
    const approvedOps = resolvedApprovals.filter((operation) => operation.approved === true).length;
    const approvalRate =
      resolvedApprovals.length > 0
        ? (approvedOps / resolvedApprovals.length) * 100
        : 0;

    return {
      turnsTotal: userTurns.length,
      naturalLanguageTurns: Math.max(0, userTurns.length - slashTurns),
      slashTurns,
      autoRoutedTurns,
      autoRouteRate: userTurns.length > 0 ? (autoRoutedTurns / userTurns.length) * 100 : 0,
      medianPromptToPlanMs: median(planLatency),
      pendingApprovals: pendingFileApprovals,
      approvalRate,
    };
  }, [messages, timeline, pendingFileApprovals]);

  useEffect(() => {
    const timer = window.setTimeout(() => {
      vscodeApi.postMessage({
        type: "uiKpiSnapshot",
        snapshot: {
          ...uiKpi,
          timestamp: Date.now(),
        },
      });
    }, 900);

    return () => window.clearTimeout(timer);
  }, [uiKpi, vscodeApi]);
  const kpiTrend = useMemo(() => {
    const summary7d = uiKpiHistory?.summary_7d;
    const summary30d = uiKpiHistory?.summary_30d;
    const safeDelta = (a: number, b: number): number => a - b;
    return {
      hasData: Boolean(summary7d && summary30d),
      autoRouteDelta:
        summary7d && summary30d ? safeDelta(summary7d.auto_route_rate, summary30d.auto_route_rate) : 0,
      promptToPlanDelta:
        summary7d && summary30d
          ? safeDelta(summary7d.median_prompt_to_plan_ms, summary30d.median_prompt_to_plan_ms)
          : 0,
      approvalDelta:
        summary7d && summary30d ? safeDelta(summary7d.approval_rate, summary30d.approval_rate) : 0,
      series14d: uiKpiHistory?.series_14d ?? [],
    };
  }, [uiKpiHistory]);

  const closeOnboarding = (persistDismiss: boolean) => {
    setOnboardingOpen(false);
    if (persistDismiss) {
      window.localStorage.setItem("sentinel.onboarding.dismissed", "true");
    }
  };

  const runOnboarding = () => {
    const goal = onboardingGoal.trim();
    const users = onboardingUsers.trim();
    if (!goal || !users) return;

    const prompt =
      `Build an app with this target outcome: ${goal}\n` +
      `Primary users: ${users}\n` +
      `Operating mode: ${onboardingMode === "no-code" ? "no-code friendly" : "developer-centric"}\n` +
      `Priority: ${onboardingPriority}\n\n` +
      "Execution requirements:\n" +
      "- outcome-first UX for end users\n" +
      "- modular and composable architecture\n" +
      "- deterministic implementation plan with safe approval gates\n" +
      "- minimal regressions and clear rollout checkpoints";

    sendChatPrompt(prompt);
    closeOnboarding(true);
    setOnboardingStep(1);
  };

  const hasExplainableMessages = useMemo(
    () =>
      messages.some(
        (message) =>
          message.role === "assistant" && (Boolean(message.explainability) || Boolean(message.thoughtChain)),
      ),
    [messages],
  );
  const missingRequiredDeps =
    Array.isArray((worldModel?.required_missing_now as any)?.dependencies)
      ? ((worldModel?.required_missing_now as any).dependencies as unknown[]).length
      : 0;
  const missingRequiredFrameworks =
    Array.isArray((worldModel?.required_missing_now as any)?.frameworks)
      ? ((worldModel?.required_missing_now as any).frameworks as unknown[]).length
      : 0;
  const workflowAssistant = useMemo(() => {
    const pendingCount = pendingGoals.length;
    const hasGovernancePending = Boolean(governance?.pending_proposal);
    const missingRequired = missingRequiredDeps + missingRequiredFrameworks;
    const qualityFailed = qualityStatus?.latest?.overall_ok === false;
    const reliabilityViolated = reliabilitySlo?.healthy === false;

    if (hasGovernancePending) {
      return {
        title: "Governance approval required",
        action: "Review pending governance proposal and approve/reject before coding.",
        command: "Open Runtime > Governance Policy",
      };
    }
    if (missingRequired > 0) {
      return {
        title: "Contract drift detected",
        action: `Resolve ${missingRequired} required governance mismatch(es) before execution.`,
        command: "Run Governance Seed Preview",
      };
    }
    if (pendingCount > 0) {
      return {
        title: "Execute next pending goal",
        action: `There are ${pendingCount} pending goals. Execute only the first pending item.`,
        command: "/execute-first-pending",
      };
    }
    if (qualityFailed || reliabilityViolated) {
      return {
        title: "Stabilize runtime quality",
        action: "No pending goals but reliability/quality is degraded. Run harness and inspect violations.",
        command: "Run Quality Harness",
      };
    }
    return {
      title: "System ready",
      action: "No pending blockers detected. Start a new goal or refine constraints.",
      command: "/init <project description>",
    };
  }, [
    pendingGoals,
    governance?.pending_proposal,
    missingRequiredDeps,
    missingRequiredFrameworks,
    qualityStatus?.latest?.overall_ok,
    reliabilitySlo?.healthy,
  ]);

  const beginChatHeightResize = (event: React.MouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    const startY = event.clientY;
    const startHeight = chatHeightRef.current;

    const onMove = (moveEvent: MouseEvent) => {
      const delta = moveEvent.clientY - startY;
      const next = Math.max(280, Math.min(window.innerHeight - 180, startHeight + delta));
      setChatMessagesHeight(next);
    };

    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  const beginTimelineResize = (event: React.MouseEvent<HTMLDivElement>) => {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = timelineWidthRef.current;

    const onMove = (moveEvent: MouseEvent) => {
      const delta = startX - moveEvent.clientX;
      const next = Math.max(240, Math.min(540, startWidth + delta));
      setTimelineWidth(next);
    };

    const onUp = () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  };

  return (
    <div
      className={cn(
        "sentinel-shell",
        simpleMode ? "sentinel-shell--chat-first" : "sentinel-shell--mission",
        `sentinel-theme--${themePreset}`,
        compactDensity && "sentinel-density--compact",
      )}
    >
      {!simpleMode && <aside className="sentinel-rail sentinel-panel-resizable">
        <div className="sentinel-brand">
          <div className="sentinel-brand__glyph">S</div>
          <div>
            <div className="sentinel-brand__title">SENTINEL</div>
            <div className="sentinel-brand__subtitle">Deterministic Agent OS</div>
          </div>
        </div>

        <nav className="sentinel-nav">
          {navItems.map((item) => {
            const Icon = item.icon;
            return (
              <button
                key={item.id}
                onClick={() => setActivePage(item.id)}
                className={cn("sentinel-nav__item", activePage === item.id && "sentinel-nav__item--active")}
              >
                <Icon className="size-4" />
                <span>{item.label}</span>
                {item.id === "chat" && messages.length > 0 && (
                  <span className="sentinel-nav__counter">{messages.length}</span>
                )}
              </button>
            );
          })}
        </nav>

        <div className="sentinel-rail__footer">
          <div className="sentinel-kv">
            <span>Connection</span>
            <span className={connected ? "sentinel-up" : "sentinel-down"}>
              {connected ? "Connected" : "Offline"}
            </span>
          </div>
          <div className="sentinel-kv">
            <span>Risk</span>
            <span className={risk.className}>{risk.label}</span>
          </div>
          <div className="sentinel-kv">
            <span>Mode</span>
            <span>Supervised</span>
          </div>
        </div>
      </aside>}

      <main className={cn("sentinel-main", simpleMode && "sentinel-main--sidebar")}>
        <header className={cn("sentinel-topbar", simpleMode && "sentinel-topbar--sidebar")}>
          <div>
            <p className="sentinel-topbar__eyebrow">{simpleMode ? "Simple Mode" : "Mission Control"}</p>
            <h1>{simpleMode ? "Sentinel Chat" : pageTitle}</h1>
          </div>
          <div className="sentinel-topbar__metrics">
            <Button
              size="xs"
              variant="outline"
              onClick={() => {
                setUiMode((prev) => (prev === "simple" ? "advanced" : "simple"));
              }}
            >
              {simpleMode ? (
                <>
                  <LayoutPanelTop className="size-3.5" />
                  Advanced
                </>
              ) : (
                <>
                  <PanelRightOpen className="size-3.5" />
                  Simple
                </>
              )}
            </Button>
            {simpleMode ? (
              <>
                <div className="sentinel-pill">
                  <Activity className="size-3.5" />
                  <span>{connected ? "Connected" : "Offline"}</span>
                </div>
                <div className="sentinel-pill">
                  <ShieldCheck className="size-3.5" />
                  <span>Outcome-first</span>
                </div>
                {pendingFileApprovals > 0 && (
                  <div className="sentinel-pill sentinel-pill--alert">
                    <ShieldAlert className="size-3.5" />
                    <span>{pendingFileApprovals} approvals</span>
                  </div>
                )}
              </>
            ) : (
              <>
                <div className="sentinel-pill">
                  <Gauge className="size-3.5" />
                  <span>Alignment {alignmentScore.toFixed(1)}%</span>
                </div>
                <div className="sentinel-pill">
                  <Activity className="size-3.5" />
                  <span>Confidence {alignmentConfidence}%</span>
                </div>
                <select
                  className="sentinel-select sentinel-select--tiny"
                  value={themePreset}
                  onChange={(event) => setThemePreset(event.target.value as ThemePreset)}
                >
                  <option value="mono-mint">Monochrome Mint</option>
                  <option value="warm-graphite">Warm Graphite</option>
                  <option value="pure-vscode">Pure VSCode</option>
                </select>
                <Button
                  size="xs"
                  variant={compactDensity ? "secondary" : "outline"}
                  onClick={() => setCompactDensity((prev) => !prev)}
                >
                  Density: {compactDensity ? "Compact" : "Comfort"}
                </Button>
                <Badge className={cn("sentinel-risk-badge", risk.className)}>{alignmentStatus}</Badge>
              </>
            )}
          </div>
        </header>

        <ScrollArea className="sentinel-content">
          <div className="sentinel-content__inner">
            {activePage === "command" && (
              <div className="sentinel-grid">
                <Card className="sentinel-card sentinel-panel-resizable sentinel-card--hero">
                  <CardHeader>
                    <CardTitle className="flex items-center gap-2 text-base">
                      <ShieldCheck className="size-4" />
                      Operational Alignment
                    </CardTitle>
                    <CardDescription>
                      Continuous enforcement of intent, invariants, reliability thresholds and governance policy.
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-5">
                    <div>
                      <div className="sentinel-progress__meta">
                        <span>Alignment Vector</span>
                        <span>{alignmentScore.toFixed(1)}%</span>
                      </div>
                      <div className="sentinel-progress">
                        <div style={{ width: `${Math.max(0, Math.min(100, alignmentScore))}%` }} />
                      </div>
                    </div>
                    <div className="sentinel-metrics">
                      <div>
                        <div>Goals Completed</div>
                        <strong>
                          {completedGoals}/{goals.length || 0}
                        </strong>
                      </div>
                      <div>
                        <div>Tool Calls</div>
                        <strong>{toolCallsCount}</strong>
                      </div>
                      <div>
                        <div>Pending Approvals</div>
                        <strong>{pendingFileApprovals}</strong>
                      </div>
                      <div>
                        <div>Drift Risk</div>
                        <strong>{risk.drift}%</strong>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                <Card className="sentinel-card sentinel-panel-resizable">
                  <CardHeader>
                    <CardTitle className="text-base">Goal Snapshot</CardTitle>
                    <CardDescription>What the agent is doing now and why.</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-3">
                    {goals.length === 0 && <p className="sentinel-empty">No goals registered yet.</p>}
                    {goals.slice(0, 5).map((goal) => (
                      <button
                        key={goal.id}
                        className="sentinel-goal-row"
                        onClick={() => setActivePage("forge")}
                        type="button"
                      >
                        <span>{goal.description}</span>
                        <span>{statusToProgress(goal.status)}%</span>
                      </button>
                    ))}
                  </CardContent>
                </Card>

                <Card className="sentinel-card sentinel-panel-resizable">
                  <CardHeader>
                    <CardTitle className="text-base">Execution Signals</CardTitle>
                    <CardDescription>Latest deterministic events from the active session.</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-2">
                    {messages.length === 0 && <p className="sentinel-empty">No events yet.</p>}
                    {messages.slice(-5).reverse().map((message) => (
                      <div key={message.id} className="sentinel-event-row">
                        <div>
                          <p>{message.role === "user" ? "User Intent" : "Agent Output"}</p>
                          <small>{new Date(message.timestamp).toLocaleTimeString()}</small>
                        </div>
                        <code>{shortFingerprint(message.id, message.timestamp)}</code>
                      </div>
                    ))}
                    {runtimeCapabilities && (
                      <div className="sentinel-event-row">
                        <div>
                          <p>Runtime Capabilities</p>
                          <small>
                            {runtimeCapabilities.server_name} v{runtimeCapabilities.server_version}
                          </small>
                        </div>
                        <code>{runtimeCapabilities.tool_count} tools</code>
                      </div>
                    )}
                  </CardContent>
                </Card>

                <Card className="sentinel-card sentinel-panel-resizable">
                  <CardHeader>
                    <CardTitle className="text-base">Workflow Assistant</CardTitle>
                    <CardDescription>Deterministic next action from manifold and runtime state.</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-2">
                    <p className="sentinel-empty">{workflowAssistant.title}</p>
                    <p>{workflowAssistant.action}</p>
                    <code>{workflowAssistant.command}</code>
                  </CardContent>
                </Card>
              </div>
            )}

            {activePage === "chat" && (
              <Card className="sentinel-card sentinel-panel-resizable sentinel-chat">
                <CardHeader className={cn(simpleMode && "sentinel-chat-header--simple")}>
                  <CardTitle className="text-base">
                    {simpleMode ? "Sentinel Chat" : "Chat Runtime"}
                  </CardTitle>
                  <CardDescription>
                    {simpleMode
                      ? "Clean output first. Technical internals only when explicitly requested."
                      : "Chat-first by default. Timeline, explainability, and governance are progressive details."}
                  </CardDescription>
                  {simpleMode ? (
                    <>
                      {onboardingOpen && (
                        <div className="sentinel-onboarding">
                          <div className="sentinel-onboarding__header">
                            <h3>No-code Starter Wizard</h3>
                            <span>Step {onboardingStep}/3</span>
                          </div>

                          {onboardingStep === 1 && (
                            <div className="sentinel-onboarding__body">
                              <p>How do you want Sentinel to guide this build?</p>
                              <div className="sentinel-inline-actions">
                                <Button
                                  size="xs"
                                  variant={onboardingMode === "no-code" ? "secondary" : "outline"}
                                  onClick={() => setOnboardingMode("no-code")}
                                >
                                  No-code guided
                                </Button>
                                <Button
                                  size="xs"
                                  variant={onboardingMode === "developer" ? "secondary" : "outline"}
                                  onClick={() => setOnboardingMode("developer")}
                                >
                                  Developer mode
                                </Button>
                              </div>
                            </div>
                          )}

                          {onboardingStep === 2 && (
                            <div className="sentinel-onboarding__body">
                              <p>What should the app do? (outcome in plain language)</p>
                              <textarea
                                className="sentinel-onboarding__textarea"
                                value={onboardingGoal}
                                onChange={(event) => setOnboardingGoal(event.target.value)}
                                placeholder="Example: A client portal for requests, invoices, and support chat."
                              />
                            </div>
                          )}

                          {onboardingStep === 3 && (
                            <div className="sentinel-onboarding__body">
                              <p>Who uses this app and what is the top priority?</p>
                              <input
                                className="sentinel-input"
                                value={onboardingUsers}
                                onChange={(event) => setOnboardingUsers(event.target.value)}
                                placeholder="Example: small team + customers"
                              />
                              <div className="sentinel-inline-actions">
                                <Button
                                  size="xs"
                                  variant={onboardingPriority === "speed" ? "secondary" : "outline"}
                                  onClick={() => setOnboardingPriority("speed")}
                                >
                                  Speed
                                </Button>
                                <Button
                                  size="xs"
                                  variant={onboardingPriority === "quality" ? "secondary" : "outline"}
                                  onClick={() => setOnboardingPriority("quality")}
                                >
                                  Quality
                                </Button>
                                <Button
                                  size="xs"
                                  variant={onboardingPriority === "compliance" ? "secondary" : "outline"}
                                  onClick={() => setOnboardingPriority("compliance")}
                                >
                                  Compliance
                                </Button>
                              </div>
                            </div>
                          )}

                          <div className="sentinel-onboarding__actions">
                            <Button
                              size="xs"
                              variant="outline"
                              onClick={() => {
                                if (onboardingStep === 1) {
                                  closeOnboarding(true);
                                  return;
                                }
                                setOnboardingStep((step) => Math.max(1, step - 1));
                              }}
                            >
                              {onboardingStep === 1 ? "Skip" : "Back"}
                            </Button>
                            {onboardingStep < 3 ? (
                              <Button
                                size="xs"
                                onClick={() => setOnboardingStep((step) => Math.min(3, step + 1))}
                              >
                                Next
                              </Button>
                            ) : (
                              <Button
                                size="xs"
                                onClick={runOnboarding}
                                disabled={!onboardingGoal.trim() || !onboardingUsers.trim()}
                              >
                                Start Build
                              </Button>
                            )}
                          </div>
                        </div>
                      )}

                      <div className="sentinel-guided-flow">
                        {(["goal", "plan", "apply"] as GuidedStage[]).map((step) => (
                          <div
                            key={step}
                            className={cn(
                              "sentinel-guided-flow__step",
                              step === guidedStage && "sentinel-guided-flow__step--active",
                              step !== guidedStage &&
                                ((step === "goal" && guidedStage !== "goal") ||
                                (step === "plan" && guidedStage === "apply")) &&
                                "sentinel-guided-flow__step--complete",
                            )}
                          >
                            <span>{step === "goal" ? "Goal" : step === "plan" ? "Plan" : "Apply"}</span>
                          </div>
                        ))}
                        <p className="sentinel-guided-flow__hint">{guidedHint}</p>
                      </div>

                      <div className="sentinel-kpi-strip">
                        <span>
                          turns
                          <strong>{uiKpi.turnsTotal}</strong>
                        </span>
                        <span>
                          auto-route
                          <strong>{uiKpi.autoRouteRate.toFixed(0)}%</strong>
                        </span>
                        <span>
                          prompt→plan
                          <strong>{Math.round(uiKpi.medianPromptToPlanMs)}ms</strong>
                        </span>
                        <span>
                          approvals
                          <strong>{uiKpi.pendingApprovals}</strong>
                        </span>
                        <span>
                          apply rate
                          <strong>{uiKpi.approvalRate.toFixed(0)}%</strong>
                        </span>
                      </div>

                      <div className="sentinel-inline-actions sentinel-chat-actions--primary">
                        <Button
                          size="xs"
                          variant="outline"
                          disabled={!connected || Boolean(activeStreamingMessage)}
                          onClick={() => sendSlashCommand("/execute-first-pending")}
                        >
                          Execute Next Goal
                        </Button>
                        <Button
                          size="xs"
                          variant="outline"
                          disabled={!connected || Boolean(activeStreamingMessage)}
                          onClick={() =>
                            sendSlashCommand(
                              "/orchestrate Harden auth + payments flow --parallel=2 --count=4 --modes=plan,build,review",
                            )
                          }
                        >
                          Orchestrate Task
                        </Button>
                        <Button
                          size="xs"
                          variant={showPreviewPanel ? "secondary" : "outline"}
                          disabled={!latestAppSpec}
                          onClick={() => setShowPreviewPanel((value) => !value)}
                        >
                          {showPreviewPanel
                            ? "Hide Preview"
                            : latestAppSpec
                              ? "Live Preview"
                              : "Preview unavailable"}
                        </Button>
                        <Button
                          size="xs"
                          variant={askWhy ? "secondary" : "outline"}
                          disabled={!hasExplainableMessages}
                          onClick={() => setAskWhy((value) => !value)}
                        >
                          {askWhy ? "Hide Why" : "Ask Why"}
                        </Button>
                        <Button
                          size="xs"
                          variant="outline"
                          onClick={() => setShowChatDetails((value) => !value)}
                        >
                          {showChatDetails ? "Less Controls" : "More Controls"}
                        </Button>
                      </div>
                      {showChatDetails && (
                        <div className="sentinel-inline-actions sentinel-chat-actions--secondary">
                          {latestAppSpec && (
                            <Button
                              size="xs"
                              variant="outline"
                              disabled={!connected || Boolean(activeStreamingMessage)}
                              onClick={() => sendSlashCommand("/appspec-refine")}
                            >
                              Refine Spec
                            </Button>
                          )}
                          {latestAppSpec && (
                            <Button
                              size="xs"
                              variant="outline"
                              disabled={!connected || Boolean(activeStreamingMessage)}
                              onClick={() => sendSlashCommand("/appspec-plan")}
                            >
                              Generate Plan
                            </Button>
                          )}
                          <Button
                            size="xs"
                            variant="outline"
                            disabled={!lastUserMessage || Boolean(activeStreamingMessage)}
                            onClick={() =>
                              lastUserMessage &&
                              vscodeApi.postMessage({
                                type: "regenerateLastResponse",
                                text: lastUserMessage.content,
                              })
                            }
                          >
                            Regenerate
                          </Button>
                          <Button
                            size="xs"
                            variant="outline"
                            disabled={!activeStreamingMessage}
                            onClick={() =>
                              activeStreamingMessage &&
                              vscodeApi.postMessage({
                                type: "cancelStreaming",
                                messageId: activeStreamingMessage.id,
                              })
                            }
                          >
                            Stop Stream
                          </Button>
                          <Button
                            size="xs"
                            variant="destructive"
                            onClick={() => vscodeApi.postMessage({ type: "clearChatMemory" })}
                          >
                            Clear Memory
                          </Button>
                        </div>
                      )}
                    </>
                  ) : (
                    <div className="sentinel-inline-actions">
                      <Button
                        size="xs"
                        variant="outline"
                        disabled={!lastUserMessage || Boolean(activeStreamingMessage)}
                        onClick={() =>
                          lastUserMessage &&
                          vscodeApi.postMessage({
                            type: "regenerateLastResponse",
                            text: lastUserMessage.content,
                          })
                        }
                      >
                        Regenerate
                      </Button>
                      <Button
                        size="xs"
                        variant="outline"
                        disabled={!activeStreamingMessage}
                        onClick={() =>
                          activeStreamingMessage &&
                          vscodeApi.postMessage({
                            type: "cancelStreaming",
                            messageId: activeStreamingMessage.id,
                          })
                        }
                      >
                        Stop Stream
                      </Button>
                      <Button
                        size="xs"
                        variant="destructive"
                        onClick={() => vscodeApi.postMessage({ type: "clearChatMemory" })}
                      >
                        Clear Memory
                      </Button>
                    </div>
                  )}
                </CardHeader>
                <CardContent className="sentinel-chat__body">
                  {simpleMode && messages.length === 0 && <QuickPrompts />}
                  <div
                    className="sentinel-chat-layout"
                    style={
                      !simpleMode
                        ? {
                            gridTemplateColumns: `minmax(0, 1fr) 8px ${timelineWidth}px`,
                          }
                        : showSimplePreview
                          ? { gridTemplateColumns: "minmax(0, 1fr) minmax(320px, 390px)" }
                        : undefined
                    }
                  >
                    <div className="sentinel-chat-primary">
                      <div
                        className="sentinel-chat__messages"
                        style={
                          simpleMode
                            ? { height: "74vh", minHeight: "620px" }
                            : { height: `${chatMessagesHeight}px` }
                        }
                      >
                        <MessageList
                          compact={compactDensity}
                          clineMode={simpleMode}
                          simpleMode={simpleMode}
                          showInternals={showInternals}
                          askWhy={askWhy}
                        />
                      </div>
                      {!simpleMode && (
                        <div
                          className="sentinel-resize-handle sentinel-resize-handle--horizontal"
                          onMouseDown={beginChatHeightResize}
                          title="Resize messages panel"
                          role="separator"
                        />
                      )}
                    </div>
                    {!simpleMode && (
                      <div
                        className="sentinel-resize-handle sentinel-resize-handle--vertical"
                        onMouseDown={beginTimelineResize}
                        title="Resize timeline panel"
                        role="separator"
                      />
                    )}
                    {showSimplePreview && latestAppSpec && (
                      <aside className="sentinel-preview-panel">
                        <Suspense fallback={<div className="sentinel-empty p-3">Loading preview...</div>}>
                          <AppSpecPreview appSpec={latestAppSpec} />
                        </Suspense>
                      </aside>
                    )}
                    {showTimelinePanel && <aside
                      className="sentinel-timeline"
                      style={
                        !simpleMode
                          ? { height: `${chatMessagesHeight}px`, maxHeight: "none" }
                          : hasSidePanel
                            ? { height: "74vh", minHeight: "620px", maxHeight: "none" }
                            : undefined
                      }
                    >
                      <div className="sentinel-timeline__header">
                        <h4>Task Timeline</h4>
                        <div className="sentinel-inline-actions">
                          <Button
                            size="icon-xs"
                            variant="outline"
                            onClick={() => setTimelineReplay((v) => !v)}
                            disabled={filteredTimeline.length === 0}
                          >
                            {timelineReplay ? <Pause className="size-3" /> : <Play className="size-3" />}
                          </Button>
                          <Button
                            size="icon-xs"
                            variant="outline"
                            onClick={() => setTimelineCursor(0)}
                            disabled={filteredTimeline.length === 0}
                          >
                            <RotateCcw className="size-3" />
                          </Button>
                          <Button
                            size="xs"
                            variant="destructive"
                            onClick={() => {
                              clearTimeline();
                              setTimelineCursor(0);
                              setTimelineReplay(false);
                            }}
                            disabled={timeline.length === 0}
                          >
                            Clear
                          </Button>
                        </div>
                        <div className="sentinel-timeline__filters">
                          <select
                            className="sentinel-select"
                            value={timelineStageFilter}
                            onChange={(event) => {
                              setTimelineStageFilter(event.target.value as TimelineStage);
                              setTimelineCursor(0);
                            }}
                          >
                            <option value="all">All stages</option>
                            <option value="received">received</option>
                            <option value="plan">plan</option>
                            <option value="tool">tool</option>
                            <option value="stream">stream</option>
                            <option value="approval">approval</option>
                            <option value="result">result</option>
                            <option value="error">error</option>
                            <option value="cancel">cancel</option>
                          </select>
                          <input
                            className="sentinel-input"
                            value={timelineTurnFilter}
                            onChange={(event) => {
                              setTimelineTurnFilter(event.target.value);
                              setTimelineCursor(0);
                            }}
                            placeholder="Filter by turnId"
                          />
                        </div>
                      </div>
                      <div className="sentinel-timeline__body">
                        {filteredTimeline.length === 0 ? (
                          <p className="sentinel-empty">No timeline events yet.</p>
                        ) : (
                          filteredTimeline
                            .slice()
                            .reverse()
                            .map((event, idxReverse) => {
                              const originalIndex = filteredTimeline.length - 1 - idxReverse;
                              const active = originalIndex === timelineCursor;
                              return (
                                <button
                                  key={event.id}
                                  type="button"
                                  className={cn("sentinel-timeline__event", active && "sentinel-timeline__event--active")}
                                  onClick={() => setTimelineCursor(originalIndex)}
                                >
                                  <div className="sentinel-timeline__meta">
                                    <span>{event.stage}</span>
                                    <small>{new Date(event.timestamp).toLocaleTimeString()}</small>
                                  </div>
                                  <strong>{event.title}</strong>
                                  {event.detail && <p>{event.detail}</p>}
                                </button>
                              );
                            })
                        )}
                      </div>
                      {currentTimelineEvent && (
                        <div className="sentinel-timeline__active">
                          <div className="sentinel-timeline__meta">
                            <span>{currentTimelineEvent.stage}</span>
                            <small>{new Date(currentTimelineEvent.timestamp).toLocaleTimeString()}</small>
                          </div>
                          <strong>{currentTimelineEvent.title}</strong>
                          {currentTimelineEvent.turnId && <p>turn: {currentTimelineEvent.turnId.slice(0, 8)}</p>}
                          {currentTimelineEvent.detail && <p>{currentTimelineEvent.detail}</p>}
                        </div>
                      )}
                    </aside>}
                  </div>
                  <div className="sentinel-chat-composer">
                    <ChatInput compact={compactDensity} clineMode={simpleMode} />
                  </div>
                </CardContent>
              </Card>
            )}

            {activePage === "forge" && (
              <div className="sentinel-grid sentinel-grid--forge">
                <Card className="sentinel-card sentinel-panel-resizable">
                  <CardHeader>
                    <CardTitle className="text-base">Goal Graph</CardTitle>
                    <CardDescription>Dependency topology and execution ordering.</CardDescription>
                  </CardHeader>
                  <CardContent className="h-[440px]">
                    <Suspense fallback={<p className="sentinel-empty">Loading goal graph...</p>}>
                      <GoalGraph goals={goals as unknown as Goal[]} onNodeSelect={setSelectedGoal} />
                    </Suspense>
                  </CardContent>
                </Card>
                <Card className="sentinel-card sentinel-panel-resizable">
                  <CardHeader>
                    <CardTitle className="text-base">Selected Goal</CardTitle>
                    <CardDescription>
                      {selectedGoal ? `Goal ${selectedGoal.id.slice(0, 8)}` : "Select a node to inspect details"}
                    </CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    {selectedGoal ? (
                      <div className="sentinel-selected-goal">
                        <p>{selectedGoal.description}</p>
                        <Badge variant="outline">{selectedGoal.status ?? "Pending"}</Badge>
                      </div>
                    ) : (
                      <p className="sentinel-empty">No goal selected.</p>
                    )}
                    <Suspense fallback={<p className="sentinel-empty">Loading goal tree...</p>}>
                      <GoalTree />
                    </Suspense>
                  </CardContent>
                </Card>
              </div>
            )}

            {activePage === "transcript" && (
              <div className="sentinel-grid">
                <Suspense fallback={<div className="sentinel-card"><p className="sentinel-empty">Loading pinned transcript...</p></div>}>
                  <PinnedTranscript />
                </Suspense>
              </div>
            )}

            {activePage === "quality" && (
              <div className="sentinel-grid">
                <Suspense fallback={<div className="sentinel-card"><p className="sentinel-empty">Loading quality dashboard...</p></div>}>
                  <QualityDashboard />
                </Suspense>
              </div>
            )}

            {activePage === "network" && (
              <Card className="sentinel-card sentinel-panel-resizable">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-base">
                    <Orbit className="size-4" />
                    Federation View
                  </CardTitle>
                  <CardDescription>
                    Cross-agent orchestration is enabled only when relay policies and trust vectors are valid.
                  </CardDescription>
                </CardHeader>
                <CardContent className="sentinel-metrics sentinel-metrics--network">
                  <div>
                    <div>Relay Health</div>
                    <strong>{connected ? "Nominal" : "Unavailable"}</strong>
                  </div>
                  <div>
                    <div>MCP Tools</div>
                    <strong>{runtimeCapabilities?.tool_count ?? 0}</strong>
                  </div>
                  <div>
                    <div>Server</div>
                    <strong>{runtimeCapabilities ? `${runtimeCapabilities.server_name}` : "n/a"}</strong>
                  </div>
                </CardContent>
              </Card>
            )}

            {activePage === "audit" && (
              <Card className="sentinel-card sentinel-panel-resizable">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-base">
                    <History className="size-4" />
                    Immutable Execution Trail
                  </CardTitle>
                  <CardDescription>Deterministic fingerprints for each user/agent turn.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-2">
                  {messages.length === 0 && <p className="sentinel-empty">Audit trail will appear after first interaction.</p>}
                  {messages.map((message) => (
                    <div key={message.id} className="sentinel-audit-row">
                      <div>
                        <p>{message.role === "user" ? "User Prompt" : "Agent Response"}</p>
                        <small>{new Date(message.timestamp).toLocaleString()}</small>
                      </div>
                      <code>{shortFingerprint(message.id, message.timestamp)}</code>
                    </div>
                  ))}
                </CardContent>
              </Card>
            )}

            {activePage === "settings" && (
              <Card className="sentinel-card sentinel-panel-resizable">
                <CardHeader>
                  <CardTitle className="flex items-center gap-2 text-base">
                    <Wrench className="size-4" />
                    Runtime Controls
                  </CardTitle>
                  <CardDescription>Current execution guardrails and user-supervision contract.</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {policyAction && (
                    <div className={cn("sentinel-policy-banner", policyAction.ok ? "sentinel-policy-banner--ok" : "sentinel-policy-banner--error")}>
                      <strong>{policyAction.kind}</strong>
                      <span>{policyAction.message}</span>
                    </div>
                  )}

                  <div className="sentinel-settings-grid">
                    <div>
                      <ShieldCheck className="size-4" />
                      <p>Action Gate</p>
                      <small>Mandatory approval for sensitive file operations.</small>
                    </div>
                    <div>
                      <ShieldAlert className="size-4" />
                      <p>Governance Lock</p>
                      <small>Deps/framework/endpoints changes require explicit approval.</small>
                    </div>
                    <div>
                      <CheckCircle2 className="size-4" />
                      <p>Reliability Thresholds</p>
                      <small>Hard stop when quality metrics drop below policy.</small>
                    </div>
                  </div>

                  <section className="sentinel-policy-card sentinel-panel-resizable">
                    <header>
                      <h3>Product KPI Telemetry</h3>
                      <Badge variant="outline">S7</Badge>
                    </header>
                    <p>Local product signals used to measure UX quality and intent-routing effectiveness.</p>
                    <div className="sentinel-kpi-strip sentinel-kpi-strip--runtime">
                      <span>
                        turns
                        <strong>{uiKpi.turnsTotal}</strong>
                      </span>
                      <span>
                        natural
                        <strong>{uiKpi.naturalLanguageTurns}</strong>
                      </span>
                      <span>
                        slash
                        <strong>{uiKpi.slashTurns}</strong>
                      </span>
                      <span>
                        auto-route
                        <strong>{uiKpi.autoRouteRate.toFixed(0)}%</strong>
                      </span>
                      <span>
                        prompt→plan
                        <strong>{Math.round(uiKpi.medianPromptToPlanMs)}ms</strong>
                      </span>
                      <span>
                        apply rate
                        <strong>{uiKpi.approvalRate.toFixed(0)}%</strong>
                      </span>
                    </div>
                    {kpiTrend.hasData && (
                      <div className="sentinel-kpi-trend-grid">
                        <div>
                          <small>7d vs 30d auto-route</small>
                          <strong className={cn(kpiTrend.autoRouteDelta >= 0 ? "sentinel-up" : "sentinel-down")}>
                            {kpiTrend.autoRouteDelta >= 0 ? "+" : ""}
                            {kpiTrend.autoRouteDelta.toFixed(1)}%
                          </strong>
                        </div>
                        <div>
                          <small>7d vs 30d prompt→plan</small>
                          <strong className={cn(kpiTrend.promptToPlanDelta <= 0 ? "sentinel-up" : "sentinel-down")}>
                            {kpiTrend.promptToPlanDelta >= 0 ? "+" : ""}
                            {Math.round(kpiTrend.promptToPlanDelta)}ms
                          </strong>
                        </div>
                        <div>
                          <small>7d vs 30d apply rate</small>
                          <strong className={cn(kpiTrend.approvalDelta >= 0 ? "sentinel-up" : "sentinel-down")}>
                            {kpiTrend.approvalDelta >= 0 ? "+" : ""}
                            {kpiTrend.approvalDelta.toFixed(1)}%
                          </strong>
                        </div>
                      </div>
                    )}
                    {kpiTrend.series14d.length > 0 && (
                      <div className="sentinel-trend-bars" aria-label="14 day auto-route trend">
                        {kpiTrend.series14d.map((point) => (
                          <div
                            key={point.date}
                            className="sentinel-trend-bars__bar"
                            style={{ height: `${Math.max(10, Math.min(100, point.auto_route_rate))}%` }}
                            title={`${point.date}: ${point.auto_route_rate.toFixed(1)}%`}
                          />
                        ))}
                      </div>
                    )}
                  </section>

                  <div className="sentinel-panel-grid">
                    <section className="sentinel-policy-card sentinel-panel-resizable">
                      <header>
                        <h3>Appearance</h3>
                        <Badge variant="outline">Parity+</Badge>
                      </header>
                      <p>Theme presets optimized for IDE-native contrast and long chat sessions.</p>
                      <div className="sentinel-inline-actions">
                        <Button
                          size="xs"
                          variant={themePreset === "mono-mint" ? "secondary" : "outline"}
                          onClick={() => setThemePreset("mono-mint")}
                        >
                          Monochrome Mint
                        </Button>
                        <Button
                          size="xs"
                          variant={themePreset === "warm-graphite" ? "secondary" : "outline"}
                          onClick={() => setThemePreset("warm-graphite")}
                        >
                          Warm Graphite
                        </Button>
                        <Button
                          size="xs"
                          variant={themePreset === "pure-vscode" ? "secondary" : "outline"}
                          onClick={() => setThemePreset("pure-vscode")}
                        >
                          Pure VSCode
                        </Button>
                      </div>
                      <label className="sentinel-toggle">
                        <input
                          type="checkbox"
                          checked={compactDensity}
                          onChange={(event) => setCompactDensity(event.target.checked)}
                        />
                        <span>Compact message density for power users</span>
                      </label>
                    </section>

                    <section className="sentinel-policy-card sentinel-panel-resizable">
                      <header>
                        <h3>Context Engine (Augment MCP)</h3>
                        <Badge variant={augmentSettings.enabled ? "outline" : "secondary"}>
                          {augmentSettings.enabled ? "Enabled" : "Disabled"}
                        </Badge>
                      </header>
                      <p>
                        Primary stack is free/local (Qdrant + filesystem/git/memory). Augment stays secondary (BYO-safe fallback).
                      </p>
                      <label className="sentinel-toggle">
                        <input
                          type="checkbox"
                          checked={augmentSettings.enabled}
                          onChange={(event) =>
                            applyAugmentSettings({
                              ...augmentSettings,
                              enabled: event.target.checked,
                            })
                          }
                        />
                        <span>Enable Augment MCP context retrieval</span>
                      </label>
                      <div className="sentinel-inline-actions">
                        <select
                          className="sentinel-select"
                          value={augmentSettings.mode}
                          onChange={(event) =>
                            applyAugmentSettings({
                              ...augmentSettings,
                              mode: event.target.value as typeof augmentSettings.mode,
                            })
                          }
                        >
                          <option value="disabled">Disabled</option>
                          <option value="internal_only">Internal only</option>
                          <option value="byo_customer">BYO customer</option>
                        </select>
                      </div>
                      <label className="sentinel-toggle">
                        <input
                          type="checkbox"
                          checked={augmentSettings.enforceByo}
                          onChange={(event) =>
                            applyAugmentSettings({
                              ...augmentSettings,
                              enforceByo: event.target.checked,
                            })
                          }
                        />
                        <span>Require BYO credentials (block platform-managed shared keys)</span>
                      </label>
                    </section>
                  </div>

                  <div className="sentinel-panel-grid">
                    <section className="sentinel-policy-card sentinel-panel-resizable">
                      <header>
                        <h3>Governance Policy</h3>
                        <Button size="xs" variant="outline" onClick={requestRuntimeRefresh}>Refresh</Button>
                      </header>
                      <p>
                        Allowed deps: <strong>{governance?.allowed_dependencies?.length ?? 0}</strong> | frameworks:{" "}
                        <strong>{governance?.allowed_frameworks?.length ?? 0}</strong> | ports:{" "}
                        <strong>{governance?.allowed_ports?.length ?? 0}</strong>
                      </p>
                      <p>
                        Pending proposal:{" "}
                        <strong>{governance?.pending_proposal?.id ? governance.pending_proposal.id.slice(0, 8) : "none"}</strong>
                      </p>
                      <p>
                        World model version:{" "}
                        <strong>{worldModel?.how_enforced?.manifold_version ?? "n/a"}</strong>{" "}
                        | required missing now:{" "}
                        <strong>{missingRequiredDeps + missingRequiredFrameworks}</strong>
                      </p>
                      {worldModel?.how_enforced?.manifold_integrity_hash && (
                        <p className="sentinel-mono">
                          hash {String(worldModel.how_enforced.manifold_integrity_hash).slice(0, 12)}…
                        </p>
                      )}

                      <div className="sentinel-inline-actions">
                        <Button
                          size="xs"
                          onClick={() => vscodeApi.postMessage({ type: "governanceApprove", note: "Approved from Runtime Controls" })}
                          disabled={!governance?.pending_proposal}
                        >
                          Approve
                        </Button>
                        <Button
                          size="xs"
                          variant="destructive"
                          onClick={() => vscodeApi.postMessage({ type: "governanceReject", reason: rejectReason })}
                          disabled={!governance?.pending_proposal}
                        >
                          Reject
                        </Button>
                      </div>
                      <input
                        className="sentinel-input"
                        value={rejectReason}
                        onChange={(event) => setRejectReason(event.target.value)}
                        placeholder="Reject reason"
                      />

                      <div className="sentinel-inline-actions">
                        <Button
                          size="xs"
                          variant="outline"
                          onClick={() =>
                            vscodeApi.postMessage({
                              type: "governanceSeed",
                              apply: false,
                              lockRequired: lockRequiredSeed,
                            })
                          }
                        >
                          Seed Preview
                        </Button>
                        <Button
                          size="xs"
                          onClick={() =>
                            vscodeApi.postMessage({
                              type: "governanceSeed",
                              apply: true,
                              lockRequired: lockRequiredSeed,
                            })
                          }
                        >
                          Apply Seed
                        </Button>
                      </div>
                      <label className="sentinel-toggle">
                        <input
                          type="checkbox"
                          checked={lockRequiredSeed}
                          onChange={(event) => setLockRequiredSeed(event.target.checked)}
                        />
                        <span>Lock required = allowed on seed apply</span>
                      </label>
                    </section>

                    <section className="sentinel-policy-card sentinel-panel-resizable">
                      <header>
                        <h3>Reliability SLO</h3>
                        <Badge variant={reliabilitySlo?.healthy ? "outline" : "destructive"}>
                          {reliabilitySlo?.healthy ? "Healthy" : "Violated"}
                        </Badge>
                      </header>
                      <p>
                        Success: <strong>{((reliability?.task_success_rate ?? 0) * 100).toFixed(1)}%</strong> | No regression:{" "}
                        <strong>{((reliability?.no_regression_rate ?? 0) * 100).toFixed(1)}%</strong>
                      </p>
                      <p>
                        Rollback: <strong>{((reliability?.rollback_rate ?? 0) * 100).toFixed(2)}%</strong> | MTTR:{" "}
                        <strong>{reliability?.avg_time_to_recover_ms ?? 0}ms</strong>
                      </p>
                      <p>
                        Thresholds: success ≥{" "}
                        <strong>{(((reliabilityThresholds?.min_task_success_rate ?? 0) as number) * 100).toFixed(0)}%</strong>, rollback ≤{" "}
                        <strong>{(((reliabilityThresholds?.max_rollback_rate ?? 0) as number) * 100).toFixed(1)}%</strong>
                      </p>
                      {reliabilitySlo?.violations?.length ? (
                        <ul className="sentinel-violations">
                          {reliabilitySlo.violations.map((violation) => (
                            <li key={violation}>{violation}</li>
                          ))}
                        </ul>
                      ) : (
                        <p className="sentinel-empty">No reliability violations.</p>
                      )}
                    </section>

                    <section className="sentinel-policy-card sentinel-panel-resizable">
                      <header>
                        <h3>Quality Harness</h3>
                        <Button
                          size="xs"
                          variant="outline"
                          onClick={() => vscodeApi.postMessage({ type: "runQualityHarness" })}
                        >
                          Run
                        </Button>
                      </header>
                      <p>
                        Last run: <strong>{qualityStatus?.latest?.run_id ?? "never"}</strong>
                      </p>
                      <p>
                        Overall:{" "}
                        <strong>
                          {qualityStatus?.latest?.overall_ok === true
                            ? "PASS"
                            : qualityStatus?.latest?.overall_ok === false
                              ? "FAIL"
                              : "n/a"}
                        </strong>{" "}
                        | Duration: <strong>{qualityStatus?.latest?.duration_sec ?? 0}s</strong>
                      </p>
                      <p>
                        Tests: <strong>{qualityStatus?.latest?.kpi?.total_tests ?? 0}</strong> | Failed:{" "}
                        <strong>{qualityStatus?.latest?.kpi?.failed ?? 0}</strong> | Pass rate:{" "}
                        <strong>
                          {typeof qualityStatus?.latest?.kpi?.pass_rate === "number"
                            ? `${(qualityStatus.latest.kpi.pass_rate * 100).toFixed(1)}%`
                            : "n/a"}
                        </strong>
                      </p>
                      {qualityStatus?.latest?.path ? (
                        <p className="sentinel-mono">{qualityStatus.latest.path}</p>
                      ) : (
                        <p className="sentinel-empty">{qualityStatus?.message ?? "No quality report available."}</p>
                      )}
                    </section>
                  </div>
                </CardContent>
              </Card>
            )}
          </div>
        </ScrollArea>
      </main>
    </div>
  );
}
