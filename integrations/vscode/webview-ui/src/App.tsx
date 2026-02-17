import React, { useEffect, useMemo, useState } from "react";
import { useStore } from "./state/store";
import { useVSCodeAPI } from "./hooks/useVSCodeAPI";
import { useMCPMessages } from "./hooks/useMCPMessages";
import {
  MessageSquare,
  Bot,
  GitBranch,
  Shield,
  Settings,
  Play,
  Pause,
  Square,
  Zap,
  Terminal,
  Eye,
  EyeOff,
  Network,
  Target,
  AlertCircle,
  Maximize2,
  Minimize2,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { Button } from "./components/ui/button";
import { Badge } from "./components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";

import MessageList from "./components/Chat/MessageList";
import ChatInput from "./components/Chat/ChatInput";
import QuickPrompts from "./components/Chat/QuickPrompts";
import { CommunicationGraph } from "./components/CommunicationGraph";
import { SwarmPanel } from "./components/Swarm/SwarmPanel";
import { ProviderConfigPanel } from "./components/ProviderConfig";
import { SplitAgentPanel } from "./components/Forge/SplitAgentPanel";
import { TopologyGraph } from "./components/Network/TopologyGraph";
import { AdvancedSettings } from "./components/Settings/AdvancedSettings";
import { LivePreviewPanel } from "./components/Preview/LivePreviewPanel";
import { ErrorBoundary } from "./components/shared/ErrorBoundary";

// Main App Component
export default function App() {
  const vscodeApi = useVSCodeAPI();
  useMCPMessages(vscodeApi);

  const connected = useStore((s) => s.connected);
  const alignment = useStore((s) => s.alignment);
  const goals = useStore((s) => s.goals);
  const messages = useStore((s) => s.messages);
  const addMessage = useStore((s) => s.addMessage);

  // Feature toggles
  const [activeFeature, setActiveFeature] = useState<"chat" | "swarm" | "split" | "network" | "settings">("chat");
  const [showPreview, setShowPreview] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [orchestrationRunning, setOrchestrationRunning] = useState(false);
  const [showFocusDetails, setShowFocusDetails] = useState(false);

  useEffect(() => {
    vscodeApi.postMessage({ type: "webviewReady" });
  }, [vscodeApi]);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const msg = event.data;
      if (!msg || typeof msg.type !== "string") return;

      if (msg.type === "showProviderConfig") {
        setActiveFeature("settings");
      }
      if (msg.type === "showBlueprints") {
        setActiveFeature("chat");
      }
    };

    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, []);

  const alignmentScore = alignment?.score ?? 0;

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

  const toggleOrchestration = () => {
    const newState = !orchestrationRunning;
    setOrchestrationRunning(newState);
    vscodeApi.postMessage({ 
      type: newState ? "startOrchestration" : "pauseOrchestration" 
    });
  };

  const stopOrchestration = () => {
    setOrchestrationRunning(false);
    vscodeApi.postMessage({ type: "stopOrchestration" });
  };

  return (
    <div className={cn("sentinel-ide", isFullscreen && "fullscreen")}>
      {/* Top Bar - Minimal Professional */}
      <header className="sentinel-topbar">
        <div className="topbar-left">
          <div className="sentinel-brand">
            <Shield className="size-5 text-primary" />
            <span className="brand-text">SENTINEL</span>
          </div>
          
          {goals.length > 0 && showFocusDetails && (
            <div className="goal-pill">
              <Target className="size-3" />
              <span className="truncate max-w-[200px]">{goals[goals.length - 1].description}</span>
            </div>
          )}
        </div>

        <div className="topbar-right">
          <div className="orchestration-controls orchestration-controls--compact">
            <Button
              size="xs"
              variant={orchestrationRunning ? "destructive" : "outline"}
              onClick={toggleOrchestration}
              disabled={!connected}
              className="gap-1.5"
            >
              {orchestrationRunning ? (
                <><Pause className="size-3" /> Pause</>
              ) : (
                <><Play className="size-3" /> Start</>
              )}
            </Button>

            {orchestrationRunning && (
              <Button size="xs" variant="outline" onClick={stopOrchestration} className="gap-1.5">
                <Square className="size-3" /> Stop
              </Button>
            )}
          </div>

          <div className="status-indicators">
            <div className={cn("status-dot", connected ? "connected" : "disconnected")} />
            <span className="status-text">{connected ? "Connected" : "Offline"}</span>
          </div>
          
          {pendingFileApprovals > 0 && (
            <Badge variant="destructive" className="gap-1">
              <AlertCircle className="size-3" />
              {pendingFileApprovals}
            </Badge>
          )}

          <Button
            size="icon"
            variant="ghost"
            onClick={() => setIsFullscreen(!isFullscreen)}
          >
            {isFullscreen ? <Minimize2 className="size-4" /> : <Maximize2 className="size-4" />}
          </Button>
        </div>
      </header>

      <div className="sentinel-body">
        {/* Left Sidebar - Icon Only */}
        <aside className="feature-sidebar feature-sidebar--icon-only">
          <button
            className={cn("feature-toggle feature-toggle--icon-only", activeFeature === "chat" && "active")}
            onClick={() => setActiveFeature("chat")}
            title="Chat"
            data-tooltip="Chat"
          >
            <MessageSquare className="size-4" />
            {messages.length > 0 && <span className="badge">{messages.length}</span>}
          </button>

          <button
            className={cn("feature-toggle feature-toggle--icon-only", activeFeature === "split" && "active")}
            onClick={() => setActiveFeature("split")}
            title="Split Agent"
            data-tooltip="Split Agent"
          >
            <GitBranch className="size-4" />
          </button>

          <button
            className={cn("feature-toggle feature-toggle--icon-only", activeFeature === "swarm" && "active")}
            onClick={() => setActiveFeature("swarm")}
            title="Swarm"
            data-tooltip="Swarm"
          >
            <Bot className="size-4" />
          </button>

          <button
            className={cn("feature-toggle feature-toggle--icon-only", activeFeature === "network" && "active")}
            onClick={() => setActiveFeature("network")}
            title="Network"
            data-tooltip="Network"
          >
            <Network className="size-4" />
          </button>

          <div className="feature-divider" />

          <button
            className={cn("feature-toggle feature-toggle--icon-only", activeFeature === "settings" && "active")}
            onClick={() => setActiveFeature("settings")}
            title="Settings"
            data-tooltip="Settings"
          >
            <Settings className="size-4" />
          </button>

          <button
            className={cn("feature-toggle feature-toggle--icon-only", showPreview && "active")}
            onClick={() => setShowPreview(!showPreview)}
            title={showPreview ? "Hide Preview" : "Show Preview"}
            data-tooltip={showPreview ? "Hide Preview" : "Show Preview"}
          >
            {showPreview ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
          </button>
        </aside>

        {/* Main Content Area */}
        <main className={cn("main-content", showPreview && "with-preview")}>
          {/* Chat View */}
          {activeFeature === "chat" && (
            <div className="chat-view">
              <div className="chat-focus-header">
                <div className="chat-focus-header__title">
                  <span>{goals.length > 0 ? "Current objective" : "Start with an outcome"}</span>
                  {goals.length > 0 ? (
                    <strong className="truncate">{goals[goals.length - 1].description}</strong>
                  ) : (
                    <strong>Describe what you want to build</strong>
                  )}
                </div>
                <div className="chat-focus-header__actions">
                  <Button size="xs" variant="outline" onClick={() => setShowFocusDetails((prev) => !prev)}>
                    {showFocusDetails ? "Hide details" : "Show details"}
                  </Button>
                </div>
              </div>
              {showFocusDetails && (
                <div className="chat-focus-meta">
                  <span>{connected ? "Connected" : "Offline"}</span>
                  <span>Alignment {alignmentScore.toFixed(0)}%</span>
                  <span>Goals {goals.length}</span>
                  <span>Pending approvals {pendingFileApprovals}</span>
                </div>
              )}
              <div className="chat-messages">
                {messages.length === 0 && (
                  <QuickPrompts
                    goalsCount={goals.length}
                    pendingApprovals={pendingFileApprovals}
                    alignmentScore={alignmentScore}
                    hasConversation={messages.length > 0}
                  />
                )}
                <MessageList compact={false} clineMode={true} simpleMode={true} showInternals={false} askWhy={false} />
              </div>
              <div className="chat-input-area">
                <ChatInput compact={false} clineMode={true} />
              </div>
            </div>
          )}

          {/* Split Agent View */}
          {activeFeature === "split" && (
            <div className="split-view">
              <ErrorBoundary label="Forge">
                <SplitAgentPanel />
              </ErrorBoundary>
            </div>
          )}

          {/* Swarm View */}
          {activeFeature === "swarm" && (
            <div className="swarm-view">
              <SwarmPanel />
            </div>
          )}

          {/* Network View */}
          {activeFeature === "network" && (
            <div className="network-view">
              <Tabs defaultValue="graph" className="h-full">
                <TabsList className="m-4">
                  <TabsTrigger value="graph">Communication Graph</TabsTrigger>
                  <TabsTrigger value="topology">Topology</TabsTrigger>
                </TabsList>
                <TabsContent value="graph" className="h-[calc(100%-60px)] m-0">
                  <ErrorBoundary label="Communication Graph">
                    <CommunicationGraph height={600} />
                  </ErrorBoundary>
                </TabsContent>
                <TabsContent value="topology" className="h-[calc(100%-60px)] m-0">
                  <ErrorBoundary label="Topology Graph">
                    <TopologyGraph height={540} />
                  </ErrorBoundary>
                </TabsContent>
              </Tabs>
            </div>
          )}

          {/* Settings View */}
          {activeFeature === "settings" && (
            <div className="settings-view">
              <Tabs defaultValue="providers" className="h-full">
                <TabsList className="m-4">
                  <TabsTrigger value="providers">Providers</TabsTrigger>
                  <TabsTrigger value="alignment">Alignment</TabsTrigger>
                  <TabsTrigger value="advanced">Advanced</TabsTrigger>
                </TabsList>
                <TabsContent value="providers" className="h-[calc(100%-60px)] overflow-auto p-4">
                  <ProviderConfigPanel />
                </TabsContent>
                <TabsContent value="alignment" className="h-[calc(100%-60px)] p-4">
                  <div className="alignment-dashboard">
                    <h3>Alignment Score: {alignmentScore.toFixed(1)}%</h3>
                    <div className="progress-bar large">
                      <div className="progress-fill" style={{ width: `${alignmentScore}%` }} />
                    </div>
                  </div>
                </TabsContent>
                <TabsContent value="advanced" className="h-[calc(100%-60px)] overflow-auto">
                  <ErrorBoundary label="Advanced Settings">
                    <AdvancedSettings />
                  </ErrorBoundary>
                </TabsContent>
              </Tabs>
            </div>
          )}
        </main>

        {/* Preview Panel — real dev server preview with viewport controls */}
        {showPreview && (
          <aside className="preview-panel">
            <ErrorBoundary label="Live Preview">
              <LivePreviewPanel />
            </ErrorBoundary>
          </aside>
        )}
      </div>

      {/* Bottom Status Bar */}
      <footer className="status-bar-bottom">
        <div className="status-left">
          <span className="status-item">
            <Terminal className="size-3" />
            Ready
          </span>
          {orchestrationRunning && (
            <span className="status-item active">
              <Zap className="size-3 animate-pulse" />
              Orchestration Active
            </span>
          )}
        </div>
        <div className="status-right">
          <span className="status-item">
            <Bot className="size-3" />
            {goals.length} Goals
          </span>
          <span className="status-item">
            <Shield className="size-3" />
            {alignmentScore.toFixed(0)}% Aligned
          </span>
        </div>
      </footer>
    </div>
  );
}
