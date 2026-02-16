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
  Eye,
  EyeOff,
  Send,
  X,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { Button } from "./components/ui/button";
import { Badge } from "./components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "./components/ui/card";
import { ScrollArea } from "./components/ui/scroll-area";

import MessageList from "./components/Chat/MessageList";
import ChatInput from "./components/Chat/ChatInput";
import QuickPrompts from "./components/Chat/QuickPrompts";
import { CommunicationGraph } from "./components/CommunicationGraph";
import { SwarmPanel } from "./components/Swarm/SwarmPanel";
import { ProviderConfigPanel } from "./components/ProviderConfig";

const RISK_LEVELS = [
  { label: "Low", min: 85, className: "sentinel-risk-low" },
  { label: "Moderate", min: 65, className: "sentinel-risk-moderate" },
  { label: "High", min: 0, className: "sentinel-risk-high" },
] as const;

export default function App() {
  const vscodeApi = useVSCodeAPI();
  useMCPMessages(vscodeApi);

  const connected = useStore((s) => s.connected);
  const alignment = useStore((s) => s.alignment);
  const goals = useStore((s) => s.goals);
  const messages = useStore((s) => s.messages);
  const reliability = useStore((s) => s.reliability);
  const governance = useStore((s) => s.governance);
  const runtimeCapabilities = useStore((s) => s.runtimeCapabilities);
  const qualityStatus = useStore((s) => s.qualityStatus);
  const addMessage = useStore((s) => s.addMessage);

  // Stati semplificati: solo 3 viste
  const [activeView, setActiveView] = useState<"chat" | "swarm" | "settings">("chat");
  const [showPreview, setShowPreview] = useState(false);
  const [showProviderConfig, setShowProviderConfig] = useState(false);

  useEffect(() => {
    vscodeApi.postMessage({ type: "webviewReady" });
  }, [vscodeApi]);

  const alignmentScore = alignment?.score ?? 0;
  const alignmentStatus = alignment?.status ?? "Initializing";

  const completedGoals = useMemo(
    () => goals.filter((goal) => goal.status.toLowerCase() === "completed").length,
    [goals]
  );

  const pendingFileApprovals = useMemo(
    () =>
      messages.reduce(
        (acc, msg) =>
          acc +
          (msg.fileOperations?.filter((op) => op.approved !== true).length ?? 0),
        0
      ),
    [messages]
  );

  const risk = useMemo(() => {
    const level = RISK_LEVELS.find((c) => alignmentScore >= c.min) ?? RISK_LEVELS[2];
    return {
      label: level.label,
      className: level.className,
    };
  }, [alignmentScore]);

  const activeStreamingMessage = useMemo(
    () =>
      [...messages].reverse().find((m) => m.role === "assistant" && m.streaming),
    [messages]
  );

  const sendChatPrompt = (text: string) => {
    if (!connected || Boolean(activeStreamingMessage)) return;
    addMessage({
      id: crypto.randomUUID(),
      role: "user",
      content: text,
      timestamp: Date.now(),
    });
    vscodeApi.postMessage({ type: "chatMessage", text });
  };

  return (
    <div className="sentinel-layout">
      {/* Header minimale */}
      <header className="sentinel-header">
        <div className="sentinel-header-brand">
          <span className="sentinel-logo">SENTINEL</span>
        </div>
        <div className="sentinel-header-status">
          <div className={cn("sentinel-status-dot", connected ? "sentinel-up" : "sentinel-down")} />
          <span>{connected ? "Connected" : "Offline"}</span>
          {pendingFileApprovals > 0 && (
            <Badge variant="destructive" className="sentinel-approval-badge">
              {pendingFileApprovals} pending
            </Badge>
          )}
        </div>
      </header>

      <div className="sentinel-body">
        {/* Sidebar icon-only: 60px fissa */}
        <nav className="sentinel-sidebar">
          <button
            className={cn("sentinel-sidebar-btn", activeView === "chat" && "active")}
            onClick={() => setActiveView("chat")}
            title="Chat"
          >
            <MessageSquare className="size-5" />
          </button>
          <button
            className={cn("sentinel-sidebar-btn", activeView === "swarm" && "active")}
            onClick={() => setActiveView("swarm")}
            title="Swarm"
          >
            <Orbit className="size-5" />
          </button>
          <button
            className={cn("sentinel-sidebar-btn", activeView === "settings" && "active")}
            onClick={() => setActiveView("settings")}
            title="Settings"
          >
            <Settings className="size-5" />
          </button>

          <div className="sentinel-sidebar-divider" />

          {/* Toggle Preview */}
          <button
            className={cn("sentinel-sidebar-btn", showPreview && "active")}
            onClick={() => setShowPreview(!showPreview)}
            title={showPreview ? "Hide Preview" : "Show Preview"}
          >
            {showPreview ? <EyeOff className="size-5" /> : <Eye className="size-5" />}
          </button>
        </nav>

        {/* Main Content */}
        <main
          className="sentinel-main"
          style={{ width: showPreview ? "60%" : "calc(100% - 60px)" }}
        >
          {activeView === "chat" && (
            <div className="sentinel-chat-view">
              {/* Goal Status Header */}
              <div className="sentinel-chat-header">
                <div className="sentinel-goal-info">
                  {goals.length > 0 ? (
                    <>
                      <span className="sentinel-goal-title">
                        {goals[goals.length - 1].description.slice(0, 60)}
                        {goals[goals.length - 1].description.length > 60 ? "..." : ""}
                      </span>
                      <Badge variant="outline" className={risk.className}>
                        {alignmentStatus} {alignmentScore.toFixed(0)}%
                      </Badge>
                    </>
                  ) : (
                    <span className="sentinel-goal-placeholder">What do you want to build?</span>
                  )}
                </div>
                <div className="sentinel-chat-actions">
                  <Button
                    size="xs"
                    variant="outline"
                    onClick={() => setShowProviderConfig(true)}
                  >
                    Providers
                  </Button>
                  <Button
                    size="xs"
                    variant="outline"
                    disabled={!connected || Boolean(activeStreamingMessage)}
                    onClick={() => sendChatPrompt("/execute-first-pending")}
                  >
                    Execute
                  </Button>
                </div>
              </div>

              {/* Messages */}
              <div className="sentinel-messages-container">
                {messages.length === 0 && <QuickPrompts />}
                <MessageList compact={false} clineMode={true} simpleMode={true} showInternals={false} askWhy={false} />
              </div>

              {/* Input fisso */}
              <div className="sentinel-chat-input-wrapper">
                <ChatInput compact={false} clineMode={true} />
              </div>
            </div>
          )}

          {activeView === "swarm" && (
            <div className="sentinel-swarm-view">
              <SwarmPanel />
            </div>
          )}

          {activeView === "settings" && (
            <div className="sentinel-settings-view">
              <ScrollArea className="sentinel-settings-scroll">
                <div className="sentinel-settings-content">
                  {/* Alignment */}
                  <Card className="sentinel-settings-card">
                    <CardHeader>
                      <CardTitle className="text-sm flex items-center gap-2">
                        <ShieldCheck className="size-4" />
                        Alignment
                      </CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-3">
                      <div className="sentinel-progress-bar">
                        <div
                          className="sentinel-progress-fill"
                          style={{ width: `${alignmentScore}%` }}
                        />
                      </div>
                      <div className="sentinel-metrics-row">
                        <span>Score: {alignmentScore.toFixed(1)}%</span>
                        <span className={risk.className}>{risk.label} Risk</span>
                      </div>
                    </CardContent>
                  </Card>

                  {/* Goals */}
                  <Card className="sentinel-settings-card">
                    <CardHeader>
                      <CardTitle className="text-sm flex items-center gap-2">
                        <Target className="size-4" />
                        Goals
                      </CardTitle>
                    </CardHeader>
                    <CardContent>
                      <div className="sentinel-metrics-row">
                        <span>Completed: {completedGoals}/{goals.length}</span>
                        <span>Pending: {goals.filter((g) => g.status.toLowerCase() === "pending").length}</span>
                      </div>
                    </CardContent>
                  </Card>

                  {/* Reliability */}
                  <Card className="sentinel-settings-card">
                    <CardHeader>
                      <CardTitle className="text-sm flex items-center gap-2">
                        <Activity className="size-4" />
                        Reliability
                      </CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-2">
                      <div className="sentinel-metrics-row">
                        <span>Success Rate</span>
                        <span>{((reliability?.task_success_rate ?? 0) * 100).toFixed(1)}%</span>
                      </div>
                      <div className="sentinel-metrics-row">
                        <span>No Regression</span>
                        <span>{((reliability?.no_regression_rate ?? 0) * 100).toFixed(1)}%</span>
                      </div>
                    </CardContent>
                  </Card>

                  {/* Quality Harness */}
                  <Card className="sentinel-settings-card">
                    <CardHeader>
                      <CardTitle className="text-sm flex items-center gap-2">
                        <CheckCircle2 className="size-4" />
                        Quality
                      </CardTitle>
                    </CardHeader>
                    <CardContent className="space-y-2">
                      <div className="sentinel-metrics-row">
                        <span>Status</span>
                        <Badge
                          variant={qualityStatus?.latest?.overall_ok ? "outline" : "destructive"}
                        >
                          {qualityStatus?.latest?.overall_ok ? "PASS" : "FAIL"}
                        </Badge>
                      </div>
                      <Button
                        size="xs"
                        variant="outline"
                        onClick={() => vscodeApi.postMessage({ type: "runQualityHarness" })}
                      >
                        Run Harness
                      </Button>
                    </CardContent>
                  </Card>

                  {/* Communication Graph */}
                  <Card className="sentinel-settings-card sentinel-graph-card">
                    <CardHeader>
                      <CardTitle className="text-sm flex items-center gap-2">
                        <Orbit className="size-4" />
                        Communication Graph
                      </CardTitle>
                    </CardHeader>
                    <CardContent>
                      <div className="sentinel-graph-container">
                        <CommunicationGraph height={300} />
                      </div>
                    </CardContent>
                  </Card>
                </div>
              </ScrollArea>
            </div>
          )}
        </main>

        {/* Preview Panel - 40% width, collassabile */}
        {showPreview && (
          <aside className="sentinel-preview-panel">
            <div className="sentinel-preview-header">
              <span>Live Preview</span>
              <div className="sentinel-preview-controls">
                <Button size="icon-xs" variant="ghost" onClick={() => setShowPreview(false)}>
                  <X className="size-4" />
                </Button>
              </div>
            </div>
            <div className="sentinel-preview-content">
              <iframe
                src="about:blank"
                className="sentinel-preview-iframe"
                title="Live Preview"
              />
            </div>
          </aside>
        )}
      </div>

      {/* Provider Config Modal */}
      {showProviderConfig && (
        <div className="sentinel-modal-overlay" onClick={() => setShowProviderConfig(false)}>
          <div className="sentinel-modal" onClick={(e) => e.stopPropagation()}>
            <div className="sentinel-modal-header">
              <h3>Configure Providers</h3>
              <Button size="icon-xs" variant="ghost" onClick={() => setShowProviderConfig(false)}>
                <X className="size-4" />
              </Button>
            </div>
            <div className="sentinel-modal-body">
              <ProviderConfigPanel />
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
