import React, { useEffect, useMemo, useState } from "react";
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
import GoalTree from "./components/Goals/GoalTree";
import GoalGraph from "./components/AtomicForge/GoalGraph";

type PageId = "command" | "chat" | "forge" | "network" | "audit" | "settings";
type TimelineStage = "all" | "received" | "plan" | "tool" | "stream" | "approval" | "result" | "error" | "cancel";
type ThemePreset = "mono-mint" | "warm-graphite" | "pure-vscode";

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

  const [activePage, setActivePage] = useState<PageId>("chat");
  const [selectedGoal, setSelectedGoal] = useState<Goal | null>(null);
  const [rejectReason, setRejectReason] = useState("Policy conflict with current project direction");
  const [lockRequiredSeed, setLockRequiredSeed] = useState(true);
  const [timelineReplay, setTimelineReplay] = useState(false);
  const [timelineCursor, setTimelineCursor] = useState(0);
  const [timelineStageFilter, setTimelineStageFilter] = useState<TimelineStage>("all");
  const [timelineTurnFilter, setTimelineTurnFilter] = useState("");
  const [sidebarMode, setSidebarMode] = useState(true);
  const [showChatDetails, setShowChatDetails] = useState(false);
  const [themePreset, setThemePreset] = useState<ThemePreset>("mono-mint");
  const [compactDensity, setCompactDensity] = useState(false);

  useEffect(() => {
    vscodeApi.postMessage({ type: "webviewReady" });
    vscodeApi.postMessage({ type: "refreshRuntimePolicies" });
  }, [vscodeApi]);

  useEffect(() => {
    const savedTheme = window.localStorage.getItem("sentinel.ui.theme");
    const savedDensity = window.localStorage.getItem("sentinel.ui.compact");
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
    window.localStorage.setItem("sentinel.ui.theme", themePreset);
  }, [themePreset]);

  useEffect(() => {
    window.localStorage.setItem("sentinel.ui.compact", compactDensity ? "true" : "false");
  }, [compactDensity]);

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

  const requestRuntimeRefresh = () => {
    vscodeApi.postMessage({ type: "refreshRuntimePolicies" });
  };

  const currentTimelineEvent =
    filteredTimeline.length > 0 ? filteredTimeline[timelineCursor] : undefined;

  return (
    <div
      className={cn(
        "sentinel-shell",
        sidebarMode ? "sentinel-shell--chat-first" : "sentinel-shell--mission",
        `sentinel-theme--${themePreset}`,
        compactDensity && "sentinel-density--compact",
      )}
    >
      {!sidebarMode && <aside className="sentinel-rail">
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

      <main className={cn("sentinel-main", sidebarMode && "sentinel-main--sidebar")}>
        <header className={cn("sentinel-topbar", sidebarMode && "sentinel-topbar--sidebar")}>
          <div>
            <p className="sentinel-topbar__eyebrow">{sidebarMode ? "Sidebar Mode" : "Mission Control"}</p>
            <h1>{pageTitle}</h1>
          </div>
          <div className="sentinel-topbar__metrics">
            <Button
              size="xs"
              variant="outline"
              onClick={() => {
                setSidebarMode((prev) => !prev);
                if (!sidebarMode && activePage !== "chat") setActivePage("chat");
              }}
            >
              {sidebarMode ? (
                <>
                  <LayoutPanelTop className="size-3.5" />
                  Mission
                </>
              ) : (
                <>
                  <PanelRightOpen className="size-3.5" />
                  Sidebar
                </>
              )}
            </Button>
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
          </div>
        </header>

        <ScrollArea className="sentinel-content">
          <div className="sentinel-content__inner">
            {activePage === "command" && (
              <div className="sentinel-grid">
                <Card className="sentinel-card sentinel-card--hero">
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

                <Card className="sentinel-card">
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

                <Card className="sentinel-card">
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
              </div>
            )}

            {activePage === "chat" && (
              <Card className="sentinel-card sentinel-chat">
                <CardHeader>
                  <CardTitle className="text-base">Chat Runtime</CardTitle>
                  <CardDescription>
                    Chat-first by default. Timeline, explainability, and governance are progressive details.
                  </CardDescription>
                  <div className="sentinel-inline-actions">
                    <Button
                      size="xs"
                      variant="outline"
                      onClick={() => setShowChatDetails((v) => !v)}
                    >
                      {showChatDetails ? "Hide Details" : "Show Details"}
                    </Button>
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
                    <Button size="xs" variant="destructive" onClick={() => vscodeApi.postMessage({ type: "clearChatMemory" })}>
                      Clear Memory
                    </Button>
                  </div>
                </CardHeader>
                <CardContent className="sentinel-chat__body">
                  {!sidebarMode && <QuickPrompts />}
                  <div className="sentinel-chat-layout">
                    <div className="sentinel-chat__messages">
                      <MessageList compact={compactDensity} clineMode={sidebarMode} />
                    </div>
                    {(showChatDetails || !sidebarMode) && <aside className="sentinel-timeline">
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
                    <ChatInput compact={compactDensity} clineMode={sidebarMode} />
                  </div>
                </CardContent>
              </Card>
            )}

            {activePage === "forge" && (
              <div className="sentinel-grid sentinel-grid--forge">
                <Card className="sentinel-card">
                  <CardHeader>
                    <CardTitle className="text-base">Goal Graph</CardTitle>
                    <CardDescription>Dependency topology and execution ordering.</CardDescription>
                  </CardHeader>
                  <CardContent className="h-[440px]">
                    <GoalGraph goals={goals as unknown as Goal[]} onNodeSelect={setSelectedGoal} />
                  </CardContent>
                </Card>
                <Card className="sentinel-card">
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
                    <GoalTree />
                  </CardContent>
                </Card>
              </div>
            )}

            {activePage === "network" && (
              <Card className="sentinel-card">
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
              <Card className="sentinel-card">
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
              <Card className="sentinel-card">
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

                  <div className="sentinel-panel-grid">
                    <section className="sentinel-policy-card">
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
                  </div>

                  <div className="sentinel-panel-grid">
                    <section className="sentinel-policy-card">
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

                    <section className="sentinel-policy-card">
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
