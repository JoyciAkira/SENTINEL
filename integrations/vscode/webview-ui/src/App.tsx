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

  const [activePage, setActivePage] = useState<PageId>("command");
  const [selectedGoal, setSelectedGoal] = useState<Goal | null>(null);

  useEffect(() => {
    vscodeApi.postMessage({ type: "webviewReady" });
  }, [vscodeApi]);

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
  const alignmentStatus = alignment?.status ?? "Bootstrapping";

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

  return (
    <div className="sentinel-shell">
      <aside className="sentinel-rail">
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
      </aside>

      <main className="sentinel-main">
        <header className="sentinel-topbar">
          <div>
            <p className="sentinel-topbar__eyebrow">Current Workspace</p>
            <h1>{pageTitle}</h1>
          </div>
          <div className="sentinel-topbar__metrics">
            <div className="sentinel-pill">
              <Gauge className="size-3.5" />
              <span>Alignment {alignmentScore.toFixed(1)}%</span>
            </div>
            <div className="sentinel-pill">
              <Activity className="size-3.5" />
              <span>Confidence {alignmentConfidence}%</span>
            </div>
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
                  </CardContent>
                </Card>
              </div>
            )}

            {activePage === "chat" && (
              <Card className="sentinel-card sentinel-chat">
                  <CardHeader>
                    <CardTitle className="text-base">Prompt -&gt; Plan -&gt; Execute -&gt; Verify</CardTitle>
                  <CardDescription>
                    Every action remains supervised and policy-gated before workspace mutation.
                  </CardDescription>
                </CardHeader>
                <CardContent className="sentinel-chat__body">
                  <QuickPrompts />
                  <div className="sentinel-chat__messages">
                    <MessageList />
                  </div>
                  <ChatInput />
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
                    <div>Active Agents</div>
                    <strong>1</strong>
                  </div>
                  <div>
                    <div>Secure Channels</div>
                    <strong>{connected ? "1/1" : "0/1"}</strong>
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
                <CardContent className="sentinel-settings-grid">
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
                </CardContent>
              </Card>
            )}
          </div>
        </ScrollArea>
      </main>
    </div>
  );
}
