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
  LayoutGrid,
  Terminal,
  Eye,
  EyeOff,
  Cpu,
  Network,
  Target,
  CheckCircle2,
  AlertCircle,
  X,
  Maximize2,
  Minimize2,
} from "lucide-react";

import { cn } from "@/lib/utils";
import { Button } from "./components/ui/button";
import { Badge } from "./components/ui/badge";
import { ScrollArea } from "./components/ui/scroll-area";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "./components/ui/tabs";

import MessageList from "./components/Chat/MessageList";
import ChatInput from "./components/Chat/ChatInput";
import QuickPrompts from "./components/Chat/QuickPrompts";
import { CommunicationGraph } from "./components/CommunicationGraph";
import { SwarmPanel } from "./components/Swarm/SwarmPanel";
import { ProviderConfigPanel } from "./components/ProviderConfig";

// Split Agent Orchestration Panel
const SplitAgentPanel: React.FC = () => {
  const [activePhase, setActivePhase] = useState<"scaffold" | "execute" | "verify">("scaffold");
  const [scaffoldProgress, setScaffoldProgress] = useState(0);
  const [modules, setModules] = useState<Array<{id: string; name: string; status: string; agent?: string}>>([]);

  return (
    <div className="split-agent-panel">
      <div className="split-agent-header">
        <div className="split-agent-title">
          <Target className="size-5 text-primary" />
          <div>
            <h3>Split Agent Orchestration</h3>
            <p>End-to-end autonomous code generation</p>
          </div>
        </div>
        <Badge variant={activePhase === "execute" ? "default" : "outline"}>
          {activePhase === "scaffold" ? "Scaffolding" : activePhase === "execute" ? "Executing" : "Verifying"}
        </Badge>
      </div>

      {/* Phase Indicator */}
      <div className="phase-indicator">
        <div className={cn("phase-step", activePhase === "scaffold" && "active", activePhase !== "scaffold" && "completed")}>
          <div className="phase-icon">1</div>
          <span>Scaffold</span>
        </div>
        <div className={cn("phase-connector", activePhase !== "scaffold" && "completed")} />
        <div className={cn("phase-step", activePhase === "execute" && "active", activePhase === "verify" && "completed")}>
          <div className="phase-icon">2</div>
          <span>Execute</span>
        </div>
        <div className={cn("phase-connector", activePhase === "verify" && "completed")} />
        <div className={cn("phase-step", activePhase === "verify" && "active")}>
          <div className="phase-icon">3</div>
          <span>Verify</span>
        </div>
      </div>

      {/* Phase 1: Scaffolding */}
      {activePhase === "scaffold" && (
        <div className="scaffold-phase">
          <div className="scaffold-description">
            <h4>🎯 Intent Interpreter Agent</h4>
            <p>Analyzing your outcome and creating atomic module scaffolding with non-negotiable boundaries...</p>
          </div>
          
          <div className="scaffold-progress">
            <div className="progress-label">
              <span>Analyzing requirements</span>
              <span>{scaffoldProgress}%</span>
            </div>
            <div className="progress-bar">
              <div className="progress-fill" style={{ width: `${scaffoldProgress}%` }} />
            </div>
          </div>

          <div className="modules-preview">
            <h4>Generated Modules</h4>
            {modules.length === 0 ? (
              <div className="modules-placeholder">
                <LayoutGrid className="size-8 text-muted" />
                <p>Modules will appear here after scaffolding completes</p>
              </div>
            ) : (
              <div className="modules-list">
                {modules.map((mod) => (
                  <div key={mod.id} className={cn("module-card", mod.status)}>
                    <div className="module-header">
                      <span className="module-name">{mod.name}</span>
                      <Badge variant="outline" size="sm">{mod.status}</Badge>
                    </div>
                    {mod.agent && (
                      <div className="module-agent">
                        <Bot className="size-3" />
                        <span>{mod.agent}</span>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      )}

      {/* Phase 2: Execution */}
      {activePhase === "execute" && (
        <div className="execute-phase">
          <div className="execute-description">
            <h4>🔧 Worker Agents</h4>
            <p>Agents working within confined modules with specific guardrails...</p>
          </div>

          <div className="active-agents">
            <div className="agent-work-card">
              <div className="agent-header">
                <Bot className="size-4 text-blue-500" />
                <span className="agent-name">AuthArchitect</span>
                <Badge size="sm">Working</Badge>
              </div>
              <div className="agent-task">Implementing JWT authentication in auth/module.ts</div>
              <div className="agent-guardrails">
                <span className="guardrail">⚡ Must use oauth2</span>
                <span className="guardrail">🔒 Secure by default</span>
              </div>
            </div>

            <div className="agent-work-card">
              <div className="agent-header">
                <Bot className="size-4 text-green-500" />
                <span className="agent-name">APICoder</span>
                <Badge size="sm" variant="secondary">Planning</Badge>
              </div>
              <div className="agent-task">Designing REST endpoints structure</div>
              <div className="agent-guardrails">
                <span className="guardrail">📐 RESTful patterns</span>
                <span className="guardrail">📝 OpenAPI spec</span>
              </div>
            </div>
          </div>

          <div className="orchestration-log">
            <h4>Orchestration Log</h4>
            <div className="log-entries">
              <div className="log-entry">
                <span className="log-time">14:32:05</span>
                <span className="log-agent">AuthArchitect</span>
                <span className="log-action">→ Completed auth/module.ts</span>
              </div>
              <div className="log-entry">
                <span className="log-time">14:32:08</span>
                <span className="log-agent">Orchestrator</span>
                <span className="log-action">→ Handoff to APICoder</span>
              </div>
              <div className="log-entry">
                <span className="log-time">14:32:10</span>
                <span className="log-agent">APICoder</span>
                <span className="log-action">→ Starting api/routes.ts</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Phase 3: Verification */}
      {activePhase === "verify" && (
        <div className="verify-phase">
          <div className="verify-description">
            <h4>✅ Verification Agent</h4>
            <p>Running end-to-end tests and quality checks...</p>
          </div>

          <div className="verification-checks">
            <div className="check-item completed">
              <CheckCircle2 className="size-4 text-green-500" />
              <span>Type checking passed</span>
            </div>
            <div className="check-item completed">
              <CheckCircle2 className="size-4 text-green-500" />
              <span>Unit tests passed (12/12)</span>
            </div>
            <div className="check-item in-progress">
              <div className="animate-spin">
                <Zap className="size-4 text-yellow-500" />
              </div>
              <span>Integration tests running...</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

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

  useEffect(() => {
    vscodeApi.postMessage({ type: "webviewReady" });
  }, [vscodeApi]);

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

  return (
    <div className={cn("sentinel-ide", isFullscreen && "fullscreen")}>
      {/* Top Bar - Minimal Professional */}
      <header className="sentinel-topbar">
        <div className="topbar-left">
          <div className="sentinel-brand">
            <Shield className="size-5 text-primary" />
            <span className="brand-text">SENTINEL</span>
          </div>
          
          {goals.length > 0 && (
            <div className="goal-pill">
              <Target className="size-3" />
              <span className="truncate max-w-[200px]">{goals[goals.length - 1].description}</span>
            </div>
          )}
        </div>

        <div className="topbar-center">
          <div className="orchestration-controls">
            <Button
              size="sm"
              variant={orchestrationRunning ? "destructive" : "default"}
              onClick={() => setOrchestrationRunning(!orchestrationRunning)}
              className="gap-2"
            >
              {orchestrationRunning ? (
                <><Pause className="size-4" /> Pause</>
              ) : (
                <><Play className="size-4" /> Start Orchestration</>
              )}
            </Button>
            
            {orchestrationRunning && (
              <Button size="sm" variant="outline" className="gap-2">
                <Square className="size-4" /> Stop
              </Button>
            )}
          </div>
        </div>

        <div className="topbar-right">
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
        {/* Left Sidebar - Feature Toggles */}
        <aside className="feature-sidebar">
          <div className="feature-group">
            <span className="feature-label">Core</span>
            
            <button
              className={cn("feature-toggle", activeFeature === "chat" && "active")}
              onClick={() => setActiveFeature("chat")}
            >
              <MessageSquare className="size-4" />
              <span>Chat</span>
              {messages.length > 0 && <span className="badge">{messages.length}</span>}
            </button>

            <button
              className={cn("feature-toggle", activeFeature === "split" && "active")}
              onClick={() => setActiveFeature("split")}
            >
              <GitBranch className="size-4" />
              <span>Split Agent</span>
              <Zap className="size-3 ml-auto text-yellow-500" />
            </button>
          </div>

          <div className="feature-group">
            <span className="feature-label">Agents</span>
            
            <button
              className={cn("feature-toggle", activeFeature === "swarm" && "active")}
              onClick={() => setActiveFeature("swarm")}
            >
              <Bot className="size-4" />
              <span>Swarm</span>
            </button>

            <button
              className={cn("feature-toggle", activeFeature === "network" && "active")}
              onClick={() => setActiveFeature("network")}
            >
              <Network className="size-4" />
              <span>Network</span>
            </button>
          </div>

          <div className="feature-group">
            <span className="feature-label">System</span>
            
            <button
              className={cn("feature-toggle", activeFeature === "settings" && "active")}
              onClick={() => setActiveFeature("settings")}
            >
              <Settings className="size-4" />
              <span>Settings</span>
            </button>

            <button
              className={cn("feature-toggle", showPreview && "active")}
              onClick={() => setShowPreview(!showPreview)}
            >
              {showPreview ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
              <span>Preview</span>
            </button>
          </div>

          {/* Mini Stats */}
          <div className="sidebar-stats">
            <div className="stat-item">
              <Cpu className="size-3" />
              <span>Alignment: {alignmentScore.toFixed(0)}%</span>
            </div>
            <div className="stat-item">
              <Target className="size-3" />
              <span>Goals: {goals.filter(g => g.status === "completed").length}/{goals.length}</span>
            </div>
          </div>
        </aside>

        {/* Main Content Area */}
        <main className={cn("main-content", showPreview && "with-preview")}>
          {/* Chat View */}
          {activeFeature === "chat" && (
            <div className="chat-view">
              <div className="chat-messages">
                {messages.length === 0 && <QuickPrompts />}
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
              <SplitAgentPanel />
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
                  <CommunicationGraph height={600} />
                </TabsContent>
                <TabsContent value="topology" className="h-[calc(100%-60px)] m-0 p-4">
                  <div className="topology-placeholder">
                    <Network className="size-16 text-muted" />
                    <p>Agent topology visualization</p>
                  </div>
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
                <TabsContent value="advanced" className="h-[calc(100%-60px)] p-4">
                  <div className="advanced-settings">
                    <p>Advanced configuration options</p>
                  </div>
                </TabsContent>
              </Tabs>
            </div>
          )}
        </main>

        {/* Preview Panel */}
        {showPreview && (
          <aside className="preview-panel">
            <div className="preview-header">
              <span>Live Preview</span>
              <Button size="icon" variant="ghost" onClick={() => setShowPreview(false)}>
                <X className="size-4" />
              </Button>
            </div>
            <div className="preview-content">
              <iframe
                src="about:blank"
                className="preview-iframe"
                title="Live Preview"
              />
            </div>
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
