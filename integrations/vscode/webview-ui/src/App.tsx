import React, { useEffect, useMemo, useState } from "react";
import { Goal } from "@sentinel/sdk";
import { useStore } from "./state/store";
import { useVSCodeAPI } from "./hooks/useVSCodeAPI";
import { useMCPMessages } from "./hooks/useMCPMessages";
import AlignmentGauge from "./components/Alignment/AlignmentGauge";
import GoalGraph from "./components/AtomicForge/GoalGraph";
import GoalBuilder from "./components/Goals/GoalBuilder";
import GoalTree from "./components/Goals/GoalTree";
import MessageList from "./components/Chat/MessageList";
import ChatInput from "./components/Chat/ChatInput";
import QuickPrompts from "./components/Chat/QuickPrompts";
import ActionPreview from "./components/Actions/ActionPreview";
import type { FileOperation, ToolCallInfo } from "./state/types";

type PageId = "command" | "chat" | "forge" | "network" | "audit" | "settings";

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

  const alignmentScore = alignment?.score ?? null;
  const alignmentConfidence = alignment?.confidence ?? null;
  const alignmentStatus = alignment?.status ?? "--";

  const goalSnapshot = useMemo(() => goals.slice(0, 4), [goals]);

  const graphGoals = useMemo<Goal[]>(() => {
    return goals.map((goal) => ({
      id: goal.id,
      description: goal.description,
      parent_id: undefined,
      dependencies: [],
      status: goal.status as Goal["status"],
      success_criteria: [],
      value_to_root: 0.1,
    }));
  }, [goals]);

  const actionGateItems = useMemo(() => {
    const tools: ToolCallInfo[] = [];
    const files: FileOperation[] = [];

    for (let i = messages.length - 1; i >= 0; i--) {
      const msg = messages[i];
      if (msg.toolCalls) tools.push(...msg.toolCalls);
      if (msg.fileOperations) files.push(...msg.fileOperations);
      if (tools.length >= 3 && files.length >= 3) break;
    }

    const items: Array<{ label: string; status: "safe" | "review"; detail: string }> = [];
    tools.slice(0, 2).forEach((tool) => {
      items.push({
        label: `Tool call: ${tool.name}`,
        status: tool.status === "success" ? "safe" : "review",
        detail: tool.status === "success" ? "Safe" : "Review",
      });
    });
    files.slice(0, 2).forEach((file) => {
      items.push({
        label: `${file.type.toUpperCase()} ${file.path}`,
        status: file.approved ? "safe" : "review",
        detail: file.approved ? "Safe" : "Review",
      });
    });

    return items.slice(0, 3);
  }, [messages]);

  const recentMessages = useMemo(() => messages.slice(-3), [messages]);

  const navItems: Array<{ id: PageId; label: string }> = [
    { id: "command", label: "Command Center" },
    { id: "chat", label: "Chat" },
    { id: "forge", label: "Atomic Forge" },
    { id: "network", label: "Network" },
    { id: "audit", label: "Audit Log" },
    { id: "settings", label: "Settings" },
  ];

  const metricToolCalls = useMemo(() => {
    let count = 0;
    for (const msg of messages) {
      if (msg.toolCalls) count += msg.toolCalls.length;
    }
    return count;
  }, [messages]);

  const metricFileOps = useMemo(() => {
    let count = 0;
    for (const msg of messages) {
      if (msg.fileOperations) count += msg.fileOperations.length;
    }
    return count;
  }, [messages]);

  return (
    <div className="shell">
      <aside className="rail">
        <div className="brand">
          <span /> Sentinel
        </div>
        <nav>
          {navItems.map((item) => (
            <button
              key={item.id}
              className={"nav-btn" + (activePage === item.id ? " active" : "")}
              data-page={item.id}
              onClick={() => setActivePage(item.id)}
            >
              {item.label}
            </button>
          ))}
        </nav>
        <div className="meta">
          <div>Mode: Supervised</div>
          <div>Policy: Guarded</div>
          <div>Agent: Local MCP</div>
        </div>
      </aside>

      <main className="main">
        <div className="topbar">
          <div className="project">
            <div>
              <h1>Sentinel Command Center</h1>
              <p>Workspace: Active</p>
            </div>
          </div>
          <div className="status">
            <div className="status-pill">
              <strong>{connected ? "Connected" : "Offline"}</strong> MCP + LSP
            </div>
            <div className="status-pill">
              Alignment <strong>{alignmentScore ? alignmentScore.toFixed(0) : "--"}%</strong>
            </div>
            <div className="status-pill">
              Confidence <strong>{alignmentConfidence ? (alignmentConfidence * 100).toFixed(0) : "--"}%</strong>
            </div>
          </div>
        </div>

        <section className={"page grid" + (activePage === "command" ? " active" : "")} id="page-command">
          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Alignment Field</h2>
                  <span>Predictive drift and recovery guidance</span>
                </div>
                <span className="chip">{alignmentStatus}</span>
              </div>
              <div className="alignment">
                <div>
                  <div className="meter">
                    <span style={{ width: `${Math.min(alignmentScore ?? 0, 100)}%` }} />
                  </div>
                  <div className="mini-grid" style={{ marginTop: 12 }}>
                    <div className="insight"><strong>Score</strong> {alignmentScore ? alignmentScore.toFixed(1) : "--"}%</div>
                    <div className="insight"><strong>Confidence</strong> {alignmentConfidence ? (alignmentConfidence * 100).toFixed(0) : "--"}%</div>
                    <div className="insight"><strong>Deviation Risk</strong> {alignmentScore ? Math.max(5, 100 - alignmentScore).toFixed(0) : "--"}%</div>
                    <div className="insight"><strong>Latency</strong> 112ms</div>
                  </div>
                </div>
                <div className="insight">
                  <strong>Next Best Action</strong>
                  <div>Finalize the action gate policy and confirm tool scopes.</div>
                  <div className="mono">Predicted impact +6.4% alignment</div>
                  <div style={{ marginTop: 10 }} className="chip">Auto-correction ready</div>
                </div>
              </div>
            </div>

            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Goal Manifold</h2>
                  <span>Current goal graph, status, blockers</span>
                </div>
                <span>{goals.length} active</span>
              </div>
              {goals.length > 0 ? (
                <div className="goal-list">
                  {goalSnapshot.map((goal) => (
                    <div key={goal.id} className="goal-item">
                      {goal.description}
                      <span>{goal.status}</span>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="empty-state">No goals synced yet.</div>
              )}
            </div>

            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Action Gate</h2>
                  <span>Pending approvals, tool safeguards</span>
                </div>
                <span className="chip">{actionGateItems.length} Pending</span>
              </div>
              <div className="action-gate">
                {actionGateItems.length === 0 ? (
                  <div className="empty-state">No pending actions.</div>
                ) : (
                  actionGateItems.map((item, index) => (
                    <div key={`${item.label}-${index}`} className="gate-row">
                      <div>{item.label}</div>
                      <span className={item.status === "safe" ? "chip" : "risk"}>{item.detail}</span>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>

          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Conversation</h2>
                  <span>Reasoning trace, tool calls</span>
                </div>
                <span>Agent v2.0</span>
              </div>
              <div className="chat">
                <div className="chat-log">
                  {recentMessages.length === 0 ? (
                    <div className="empty-state">No conversation yet.</div>
                  ) : (
                    recentMessages.map((msg) => (
                      <div key={msg.id} className="bubble">
                        <strong>{msg.role}</strong>
                        {msg.content}
                      </div>
                    ))
                  )}
                </div>
                <div className="tool-grid">
                  <div className="tool-item">MCP Tool <code>get_alignment</code></div>
                  <div className="tool-item">MCP Tool <code>validate_action</code></div>
                  <div className="tool-item">MCP Tool <code>suggest_goals</code></div>
                  <div className="tool-item">MCP Tool <code>safe_write</code></div>
                </div>
              </div>
            </div>

            <div className="card">
              <div className="card-header">
                <div>
                  <h2>System Pulse</h2>
                  <span>Telemetry, guardrails, activity</span>
                </div>
                <span>Last 60s</span>
              </div>
              <div className="insight">
                <div><strong>Policy breaches</strong> 0</div>
                <div><strong>Tool calls</strong> {metricToolCalls}</div>
                <div><strong>Files touched</strong> {metricFileOps}</div>
                <div><strong>Latency avg</strong> 118ms</div>
              </div>
            </div>

            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Runbook</h2>
                  <span>Operational guidance</span>
                </div>
                <span>v2.0</span>
              </div>
              <div className="insight">
                <div><strong>Next checkpoint</strong> Approve safe write policy.</div>
                <div><strong>Suggested test</strong> `npm run build:webview`</div>
                <div><strong>Last alert</strong> None</div>
              </div>
            </div>
          </div>
        </section>

        <section className={"page split" + (activePage === "chat" ? " active" : "")} id="page-chat">
          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Sentinel Chat</h2>
                  <span>Guided planning and tool execution</span>
                </div>
                <div className="tabs">
                  <button>Plan</button>
                  <button>Execute</button>
                  <button>Review</button>
                </div>
              </div>
              <div className="chat-panel">
                <MessageList />
                <QuickPrompts />
                <ChatInput />
              </div>
            </div>
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Action Preview</h2>
                  <span>Files, commands, and diffs</span>
                </div>
                <span>{messages.length} items</span>
              </div>
              <ActionPreview />
            </div>
          </div>

          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Toolbox</h2>
                  <span>Active MCP tools & permissions</span>
                </div>
                <span>Guarded</span>
              </div>
              <div className="tool-grid">
                <div className="tool-item">Tool <code>get_alignment</code> <span className="chip">Allowed</span></div>
                <div className="tool-item">Tool <code>safe_write</code> <span className="risk">Approval</span></div>
                <div className="tool-item">Tool <code>run_command</code> <span className="chip">Allowed</span></div>
                <div className="tool-item">Tool <code>edit_file</code> <span className="risk">Approval</span></div>
              </div>
            </div>
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Alignment Snapshot</h2>
                  <span>Live drift monitoring</span>
                </div>
                <span className="chip">{alignmentStatus}</span>
              </div>
              <AlignmentGauge />
            </div>
          </div>
        </section>

        <section className={"page grid" + (activePage === "forge" ? " active" : "")} id="page-forge">
          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Atomic Forge</h2>
                  <span>Graph view and goal topology</span>
                </div>
                <span>Graph Mode</span>
              </div>
              <div className="forge-canvas">
                <GoalGraph goals={graphGoals} onNodeSelect={setSelectedGoal} />
              </div>
            </div>
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Goal Details</h2>
                  <span>Selected node summary</span>
                </div>
                <span>{selectedGoal ? selectedGoal.id.slice(0, 6) : "Active"}</span>
              </div>
              {selectedGoal ? (
                <div className="insight">
                  <div><strong>Goal</strong> {selectedGoal.description}</div>
                  <div><strong>Status</strong> {selectedGoal.status}</div>
                  <div><strong>Dependencies</strong> {selectedGoal.dependencies.length || 0}</div>
                  <div><strong>Owner</strong> Local Agent</div>
                </div>
              ) : goals.length > 0 ? (
                <GoalTree />
              ) : (
                <div className="empty-state">No goals synced yet.</div>
              )}
            </div>
          </div>

          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Goal Builder</h2>
                  <span>Define, split, and validate goals</span>
                </div>
                <span>Draft</span>
              </div>
              <GoalBuilder />
            </div>
          </div>
        </section>

        <section className={"page grid" + (activePage === "network" ? " active" : "")} id="page-network">
          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Federation</h2>
                  <span>Peer discovery and gossip</span>
                </div>
                <span>4 nodes</span>
              </div>
              <div className="network-grid">
                <div className="settings-item"><strong>Node Alpha</strong> Healthy · 22ms</div>
                <div className="settings-item"><strong>Node Beta</strong> Healthy · 31ms</div>
                <div className="settings-item"><strong>Node Gamma</strong> Syncing · 54ms</div>
                <div className="settings-item"><strong>Node Delta</strong> Idle · 12ms</div>
              </div>
            </div>
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Consensus</h2>
                  <span>Pending proposals</span>
                </div>
                <span>2 votes</span>
              </div>
              <div className="audit-log">
                <div className="audit-item">Proposal: tighten safe_write policy <span className="chip">Vote</span></div>
                <div className="audit-item">Proposal: enable auto-fix for invariants <span className="chip">Vote</span></div>
              </div>
            </div>
          </div>

          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Threat Broadcasts</h2>
                  <span>Zero-trust alerts</span>
                </div>
                <span>0 active</span>
              </div>
              <div className="insight">
                <div><strong>No active alerts.</strong> All nodes within policy.</div>
              </div>
            </div>
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Network Telemetry</h2>
                  <span>Throughput and latency</span>
                </div>
                <span>Live</span>
              </div>
              <div className="insight">
                <div><strong>Messages/min</strong> 420</div>
                <div><strong>Avg latency</strong> 28ms</div>
                <div><strong>Packet loss</strong> 0.2%</div>
              </div>
            </div>
          </div>
        </section>

        <section className={"page grid" + (activePage === "audit" ? " active" : "")} id="page-audit">
          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Audit Log</h2>
                  <span>Immutable action history</span>
                </div>
                <span>Today</span>
              </div>
              <div className="audit-log">
                {messages.length === 0 ? (
                  <div className="empty-state">No actions recorded yet.</div>
                ) : (
                  recentMessages.map((msg) => (
                    <div key={`audit-${msg.id}`} className="audit-item">
                      {new Date(msg.timestamp).toLocaleTimeString()} - {msg.role}
                      <span className="chip">Logged</span>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>

          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Context Snapshot</h2>
                  <span>Files & memory used</span>
                </div>
                <span>Last 24h</span>
              </div>
              <div className="goal-list">
                <div className="goal-item">`integrations/vscode/src/App.tsx` <span>Read</span></div>
                <div className="goal-item">`webview-ui/src/styles.css` <span>Modified</span></div>
                <div className="goal-item">`sentinel.json` <span>Referenced</span></div>
              </div>
            </div>
          </div>
        </section>

        <section className={"page grid" + (activePage === "settings" ? " active" : "")} id="page-settings">
          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Settings</h2>
                  <span>Policy and runtime preferences</span>
                </div>
                <span>Local</span>
              </div>
              <div className="settings-grid">
                <div className="settings-item">
                  <strong>Approval Mode</strong>
                  Manual approval for file writes and shell commands.
                </div>
                <div className="settings-item">
                  <strong>Polling Interval</strong>
                  60 seconds. Configure for lower latency.
                </div>
                <div className="settings-item">
                  <strong>Memory Strategy</strong>
                  Persistent + local embeddings (Qdrant).
                </div>
              </div>
            </div>
          </div>

          <div className="stack">
            <div className="card">
              <div className="card-header">
                <div>
                  <h2>Integrations</h2>
                  <span>MCP + LSP connections</span>
                </div>
                <span>Active</span>
              </div>
              <div className="insight">
                <div><strong>Sentinel MCP</strong> ws://localhost:8000</div>
                <div><strong>Rust Analyzer</strong> Running</div>
                <div><strong>TypeScript</strong> Running</div>
              </div>
            </div>
          </div>
        </section>

        <div className="footer">Sentinel Command Center · World-class UI</div>
      </main>
    </div>
  );
}
