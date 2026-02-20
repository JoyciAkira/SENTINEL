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
  ChevronUp,
  ChevronDown,
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

// Main App Component - Chat-First Layout
export default function App() {
  const vscodeApi = useVSCodeAPI();
  useMCPMessages(vscodeApi);

  const connected = useStore((s) => s.connected);
  const alignment = useStore((s) => s.alignment);
  const goals = useStore((s) => s.goals);
  const messages = useStore((s) => s.messages);
  const addMessage = useStore((s) => s.addMessage);

  // Feature toggles - Chat-first: chat is default
  const [activeFeature, setActiveFeature] = useState<"chat" | "swarm" | "split" | "network" | "settings">("chat");
  const [showPreview, setShowPreview] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [orchestrationRunning, setOrchestrationRunning] = useState(false);
  const [showGoalDetails, setShowGoalDetails] = useState(false);

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
    <div className={cn("sentinel-ide sentinel-ide--chat-first", isFullscreen && "fullscreen")}>
      {/* Minimal Header - Brand + Status Only */}
      <header className="sentinel-header-minimal">
        <div className="header-brand">
          <Shield className="size-4 text-primary" />
          <span className="brand-text">SENTINEL</span>
        </div>

        <div className="header-center">
          {goals.length > 0 && (
            <button 
              className="goal-pill-compact"
              onClick={() => setShowGoalDetails(!showGoalDetails)}
            >
              <Target className="size-3" />
              <span className="truncate max-w-[180px]">{goals[goals.length - 1].description}</span>
              {showGoalDetails ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
            </button>
          )}
        </div>

        <div className="header-actions">
          <div className="status-indicator">
            <div className={cn("status-dot", connected ? "connected" : "disconnected")} />
          </div>
          
          {pendingFileApprovals > 0 && (
            <Badge variant="destructive" className="gap-1 text-[10px] px-2">
              <AlertCircle className="size-3" />
              {pendingFileApprovals}
            </Badge>
          )}

          <Button
            size="icon"
            variant="ghost"
            className="size-7"
            onClick={() => setIsFullscreen(!isFullscreen)}
          >
            {isFullscreen ? <Minimize2 className="size-3.5" /> : <Maximize2 className="size-3.5" />}
          </Button>
        </div>
      </header>

      {/* Goal Details Expandable */}
      {showGoalDetails && goals.length > 0 && (
        <div className="goal-details-bar">
          <div className="goal-details-content">
            <span className="detail-item">
              <span className="detail-label">Alignment</span>
              <span className="detail-value">{alignmentScore.toFixed(0)}%</span>
            </span>
            <span className="detail-item">
              <span className="detail-label">Goals</span>
              <span className="detail-value">{goals.length}</span>
            </span>
            <span className="detail-item">
              <span className="detail-label">Pending</span>
              <span className="detail-value">{pendingFileApprovals}</span>
            </span>
          </div>
          <div className="orchestration-mini">
            <Button
              size="xs"
              variant={orchestrationRunning ? "destructive" : "outline"}
              onClick={toggleOrchestration}
              disabled={!connected}
              className="gap-1 h-6 text-[10px]"
            >
              {orchestrationRunning ? <><Pause className="size-2.5" /> Pause</> : <><Play className="size-2.5" /> Start</>}
            </Button>
            {orchestrationRunning && (
              <Button size="xs" variant="outline" onClick={stopOrchestration} className="gap-1 h-6 text-[10px]">
                <Square className="size-2.5" /> Stop
              </Button>
            )}
          </div>
        </div>
      )}

      {/* Main Content - Full Width */}
      <main className="sentinel-main-full">
        {/* Chat View - Default */}
        {activeFeature === "chat" && (
          <div className="chat-view-full">
            <div className="chat-messages-full">
              {messages.length === 0 && (
                <QuickPrompts
                  goalsCount={goals.length}
                  pendingApprovals={pendingFileApprovals}
                  alignmentScore={alignmentScore}
                  hasConversation={messages.length > 0}
                />
              )}
              <MessageList />
            </div>
            <div className="chat-input-sticky">
              <ChatInput compact={false} clineMode={true} />
            </div>
          </div>
        )}

        {/* Split Agent View */}
        {activeFeature === "split" && (
          <div className="feature-view">
            <ErrorBoundary label="Forge">
              <SplitAgentPanel />
            </ErrorBoundary>
          </div>
        )}

        {/* Swarm View */}
        {activeFeature === "swarm" && (
          <div className="feature-view">
            <SwarmPanel />
          </div>
        )}

        {/* Network View */}
        {activeFeature === "network" && (
          <div className="feature-view">
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
          <div className="feature-view">
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

        {/* Preview Panel - Collapsible from right */}
        {showPreview && (
          <aside className="preview-panel-right">
            <div className="preview-header">
              <span className="text-xs font-medium">Live Preview</span>
              <Button size="icon" variant="ghost" className="size-6" onClick={() => setShowPreview(false)}>
                <EyeOff className="size-3" />
              </Button>
            </div>
            <ErrorBoundary label="Live Preview">
              <LivePreviewPanel />
            </ErrorBoundary>
          </aside>
        )}
      </main>

      {/* Bottom Navigation Tabs - Chat First */}
      <nav className="sentinel-bottom-nav">
        <div className="nav-tabs">
          <button
            className={cn("nav-tab", activeFeature === "chat" && "active")}
            onClick={() => setActiveFeature("chat")}
          >
            <MessageSquare className="size-4" />
            <span>Chat</span>
            {messages.length > 0 && <span className="tab-badge">{messages.length}</span>}
          </button>

          <button
            className={cn("nav-tab", activeFeature === "split" && "active")}
            onClick={() => setActiveFeature("split")}
          >
            <GitBranch className="size-4" />
            <span>Split</span>
          </button>

          <button
            className={cn("nav-tab", activeFeature === "swarm" && "active")}
            onClick={() => setActiveFeature("swarm")}
          >
            <Bot className="size-4" />
            <span>Swarm</span>
          </button>

          <button
            className={cn("nav-tab", activeFeature === "network" && "active")}
            onClick={() => setActiveFeature("network")}
          >
            <Network className="size-4" />
            <span>Network</span>
          </button>

          <div className="nav-divider" />

          <button
            className={cn("nav-tab", showPreview && "active")}
            onClick={() => setShowPreview(!showPreview)}
          >
            {showPreview ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
            <span>Preview</span>
          </button>

          <button
            className={cn("nav-tab", activeFeature === "settings" && "active")}
            onClick={() => setActiveFeature("settings")}
          >
            <Settings className="size-4" />
            <span>Settings</span>
          </button>
        </div>

        <div className="nav-status">
          {orchestrationRunning && (
            <span className="status-active">
              <Zap className="size-3 animate-pulse" />
              Running
            </span>
          )}
          <span className="status-info">
            <Shield className="size-3" />
            {alignmentScore.toFixed(0)}%
          </span>
        </div>
      </nav>
    </div>
  );
}