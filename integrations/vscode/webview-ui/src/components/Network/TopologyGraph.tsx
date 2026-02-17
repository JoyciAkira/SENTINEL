/**
 * TopologyGraph — Visualizzazione del Goal DAG come grafo ReactFlow.
 *
 * I nodi sono costruiti a partire dai `goals` reali dello store Zustand.
 * Il layout è topologico (sinistra→destra) basato sull'ordine dei goal.
 * Il colore di ogni nodo riflette lo stato reale del goal (pending/
 * in_progress/completed/failed). Aggiornamento reattivo: ogni volta
 * che `goals` cambia nel store, il grafo si ridisegna automaticamente.
 */
import React, { useCallback, useEffect, useMemo } from "react";
import ReactFlow, {
  Background,
  BackgroundVariant,
  Controls,
  MiniMap,
  Node,
  Edge,
  MarkerType,
  useNodesState,
  useEdgesState,
  Handle,
  Position,
  NodeProps,
} from "reactflow";
import "reactflow/dist/style.css";
import { RefreshCw, GitBranch, CheckCircle2, AlertCircle, Loader2, Circle } from "lucide-react";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";
import type { GoalNodeState } from "../../state/types";

// ─── status → visual mapping ─────────────────────────────────────────────────

interface StatusStyle {
  border: string;
  bg: string;
  text: string;
  badge: string;
  icon: React.ReactNode;
  minimap: string;
}

const STATUS_STYLES: Record<string, StatusStyle> = {
  pending:     { border: "#3f3f46", bg: "#18181b", text: "#a1a1aa", badge: "#3f3f46",     icon: <Circle       className="size-3 shrink-0" />,                     minimap: "#52525b" },
  ready:       { border: "#2563eb", bg: "#172554", text: "#93c5fd", badge: "#1d4ed8",     icon: <GitBranch    className="size-3 shrink-0" />,                     minimap: "#3b82f6" },
  in_progress: { border: "#ca8a04", bg: "#1c1917", text: "#fbbf24", badge: "#92400e",     icon: <Loader2      className="size-3 shrink-0 animate-spin" />,         minimap: "#f59e0b" },
  validating:  { border: "#ca8a04", bg: "#1c1917", text: "#fbbf24", badge: "#92400e",     icon: <Loader2      className="size-3 shrink-0 animate-spin" />,         minimap: "#f59e0b" },
  completed:   { border: "#16a34a", bg: "#052e16", text: "#4ade80", badge: "#14532d",     icon: <CheckCircle2 className="size-3 shrink-0" />,                     minimap: "#22c55e" },
  failed:      { border: "#dc2626", bg: "#1c0a0a", text: "#f87171", badge: "#7f1d1d",     icon: <AlertCircle  className="size-3 shrink-0" />,                     minimap: "#ef4444" },
  blocked:     { border: "#7c3aed", bg: "#1e1b4b", text: "#c4b5fd", badge: "#4c1d95",     icon: <AlertCircle  className="size-3 shrink-0" />,                     minimap: "#8b5cf6" },
};

function getStyle(status: string): StatusStyle {
  return STATUS_STYLES[status] ?? STATUS_STYLES["pending"];
}

// ─── custom node ─────────────────────────────────────────────────────────────

interface GoalNodeData {
  goal: GoalNodeState;
  index: number;
}

const GoalNode: React.FC<NodeProps<GoalNodeData>> = ({ data, selected }) => {
  const style = getStyle(data.goal.status);

  return (
    <div
      style={{
        border: `1px solid ${selected ? "#38bdf8" : style.border}`,
        background: style.bg,
        borderRadius: 10,
        padding: "8px 12px",
        minWidth: 160,
        maxWidth: 220,
        boxShadow: selected
          ? "0 0 0 2px #38bdf8, 0 8px 24px rgba(0,0,0,0.5)"
          : "0 4px 16px rgba(0,0,0,0.4)",
        transition: "box-shadow 150ms ease, border-color 150ms ease",
        cursor: "default",
      }}
    >
      <Handle type="target" position={Position.Left}  style={{ background: style.border, width: 8, height: 8, borderRadius: 4 }} />
      <Handle type="source" position={Position.Right} style={{ background: style.border, width: 8, height: 8, borderRadius: 4 }} />

      {/* Index pill + status icon */}
      <div style={{ display: "flex", alignItems: "center", gap: 6, marginBottom: 6 }}>
        <div
          style={{
            background: style.badge,
            color: style.text,
            borderRadius: 4,
            padding: "1px 5px",
            fontSize: 9,
            fontWeight: 700,
            fontFamily: "monospace",
            letterSpacing: "0.04em",
          }}
        >
          G{data.index + 1}
        </div>
        <div style={{ color: style.text, display: "flex", alignItems: "center" }}>
          {style.icon}
        </div>
        <div
          style={{
            marginLeft: "auto",
            fontSize: 9,
            textTransform: "uppercase",
            letterSpacing: "0.06em",
            color: style.text,
            fontWeight: 600,
          }}
        >
          {data.goal.status}
        </div>
      </div>

      {/* Description */}
      <p
        style={{
          margin: 0,
          fontSize: 11,
          fontWeight: 500,
          color: "#f4f4f5",
          lineHeight: 1.35,
          wordBreak: "break-word",
          display: "-webkit-box",
          WebkitLineClamp: 3,
          WebkitBoxOrient: "vertical",
          overflow: "hidden",
        }}
      >
        {data.goal.description}
      </p>

      {/* ID */}
      <p
        style={{
          margin: "6px 0 0",
          fontSize: 9,
          fontFamily: "monospace",
          color: "#52525b",
        }}
      >
        {data.goal.id.slice(0, 8)}
      </p>
    </div>
  );
};

const nodeTypes = { goalNode: GoalNode };

// ─── layout helpers ──────────────────────────────────────────────────────────

const NODE_W = 220;
const NODE_H = 90;
const COL_GAP = 80;
const ROW_GAP = 20;

/**
 * Layered layout: distribute goals into columns of ~3 rows.
 * Since GoalNodeState has no explicit dependency edges, we lay them
 * out sequentially in a left-to-right grid (columns of 3).
 * If alignment data is later added (e.g. dependency IDs), this can be
 * swapped for a proper topological sort.
 */
function buildLayout(goals: GoalNodeState[]): { nodes: Node[]; edges: Edge[] } {
  const ROWS_PER_COL = 3;
  const nodes: Node[] = goals.map((g, i) => {
    const col = Math.floor(i / ROWS_PER_COL);
    const row = i % ROWS_PER_COL;
    return {
      id: g.id,
      type: "goalNode",
      position: {
        x: col * (NODE_W + COL_GAP),
        y: row * (NODE_H + ROW_GAP),
      },
      data: { goal: g, index: i },
    };
  });

  // Sequential edges: G[i] → G[i+1] for adjacent goals
  const edges: Edge[] = goals.slice(0, -1).map((g, i) => ({
    id:     `e-${g.id}-${goals[i + 1].id}`,
    source: g.id,
    target: goals[i + 1].id,
    animated: goals[i].status === "in_progress" || goals[i].status === "validating",
    style: {
      stroke: getStyle(goals[i].status).border,
      strokeWidth: 1.5,
      opacity: 0.6,
    },
    markerEnd: {
      type: MarkerType.ArrowClosed,
      color: getStyle(goals[i].status).border,
      width: 14,
      height: 14,
    },
  }));

  return { nodes, edges };
}

// ─── main component ──────────────────────────────────────────────────────────

interface TopologyGraphProps {
  height?: number;
}

export const TopologyGraph: React.FC<TopologyGraphProps> = ({ height = 480 }) => {
  const vscode    = useVSCodeAPI();
  const goals     = useStore((s) => s.goals);
  const alignment = useStore((s) => s.alignment);

  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);

  // Rebuild graph whenever goals change
  useEffect(() => {
    if (goals.length === 0) {
      setNodes([]);
      setEdges([]);
      return;
    }
    const { nodes: n, edges: e } = buildLayout(goals);
    setNodes(n);
    setEdges(e);
  }, [goals, setNodes, setEdges]);

  const refresh = useCallback(() => {
    vscode.postMessage({ type: "chatMessage", text: "get_goal_graph" });
  }, [vscode]);

  // Legend counts
  const counts = useMemo(() => ({
    total:       goals.length,
    completed:   goals.filter((g) => g.status === "completed").length,
    in_progress: goals.filter((g) => g.status === "in_progress" || g.status === "validating").length,
    pending:     goals.filter((g) => g.status === "pending" || g.status === "ready").length,
    failed:      goals.filter((g) => g.status === "failed").length,
  }), [goals]);

  if (goals.length === 0) {
    return (
      <div
        style={{ height }}
        className="flex flex-col items-center justify-center gap-4 text-muted-foreground"
      >
        <GitBranch className="size-12 opacity-20" />
        <p className="text-sm">No goals found in current manifold.</p>
        <Button size="sm" variant="outline" onClick={refresh} className="gap-2">
          <RefreshCw className="size-3.5" /> Load Goal Graph
        </Button>
      </div>
    );
  }

  return (
    <div style={{ height, display: "flex", flexDirection: "column" }}>
      {/* Toolbar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "8px 12px",
          borderBottom: "1px solid var(--sentinel-border)",
          background: "color-mix(in oklab, var(--sentinel-surface) 95%, transparent)",
          flexShrink: 0,
        }}
      >
        {/* Legend chips */}
        <div style={{ display: "flex", gap: 6, flexWrap: "wrap", flex: 1 }}>
          {[
            { status: "completed",   count: counts.completed,   label: "Done" },
            { status: "in_progress", count: counts.in_progress, label: "Active" },
            { status: "pending",     count: counts.pending,     label: "Pending" },
            { status: "failed",      count: counts.failed,      label: "Failed" },
          ].map(({ status, count, label }) => (
            <div
              key={status}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 5,
                border: `1px solid ${getStyle(status).border}`,
                borderRadius: 999,
                padding: "2px 8px",
                fontSize: 10,
                color: getStyle(status).text,
                background: getStyle(status).bg,
              }}
            >
              <div
                style={{
                  width: 6,
                  height: 6,
                  borderRadius: "50%",
                  background: getStyle(status).minimap,
                }}
              />
              {count} {label}
            </div>
          ))}
        </div>

        {alignment && (
          <span
            style={{
              fontSize: 10,
              color: "var(--sentinel-muted)",
              fontVariantNumeric: "tabular-nums",
            }}
          >
            Alignment {alignment.score.toFixed(1)}%
          </span>
        )}

        <Button size="sm" variant="ghost" onClick={refresh} className="gap-1.5 h-7 shrink-0">
          <RefreshCw className="size-3" /> Refresh
        </Button>
      </div>

      {/* Graph canvas */}
      <div style={{ flex: 1, position: "relative" }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          nodeTypes={nodeTypes}
          fitView
          fitViewOptions={{ padding: 0.25 }}
          proOptions={{ hideAttribution: true }}
          minZoom={0.3}
          maxZoom={1.5}
          style={{ background: "transparent" }}
        >
          <Background
            variant={BackgroundVariant.Dots}
            gap={18}
            size={1}
            color="var(--sentinel-border)"
            style={{ opacity: 0.4 }}
          />
          <Controls
            style={{
              background: "color-mix(in oklab, var(--sentinel-surface) 92%, transparent)",
              border: "1px solid var(--sentinel-border)",
              borderRadius: 8,
            }}
          />
          <MiniMap
            nodeColor={(n) => getStyle((n.data as GoalNodeData).goal?.status ?? "pending").minimap}
            maskColor="rgba(0,0,0,0.3)"
            style={{
              background: "color-mix(in oklab, var(--sentinel-surface) 90%, transparent)",
              border: "1px solid var(--sentinel-border)",
              borderRadius: 8,
            }}
          />
        </ReactFlow>
      </div>
    </div>
  );
};

export default TopologyGraph;
