/**
 * SplitAgentPanel — Orchestrazione atomica degli obiettivi in 3 fasi.
 *
 * Phase 1 (Scaffold): Legge i goal reali dallo store Zustand. Se assenti,
 *   permette l'inizializzazione del progetto via `/init`. Invia comandi MCP
 *   reali (`getGoalGraph`) per popolare la lista moduli.
 *
 * Phase 2 (Execute): Avvia l'esecuzione del primo goal pendente tramite
 *   `chatMessage`. Visualizza il timeline dello store come log live.
 *
 * Phase 3 (Verify): Richiede il quality harness tramite MCP e visualizza
 *   il risultato da `qualityStatus`.
 */
import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Target,
  CheckCircle2,
  Circle,
  AlertCircle,
  Clock,
  Play,
  ArrowRight,
  RefreshCw,
  TerminalSquare,
  GitBranch,
  ShieldCheck,
  Loader2,
  ChevronRight,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";
import type { GoalNodeState, TimelineEventState } from "../../state/types";

// ─── helpers ────────────────────────────────────────────────────────────────

type Phase = "scaffold" | "execute" | "verify";

const PHASE_LABELS: Record<Phase, string> = {
  scaffold: "Scaffold",
  execute:  "Execute",
  verify:   "Verify",
};

const PHASE_ICONS: Record<Phase, React.ReactNode> = {
  scaffold: <GitBranch className="size-3.5" />,
  execute:  <Play       className="size-3.5" />,
  verify:   <ShieldCheck className="size-3.5" />,
};

const STATUS_META: Record<string, { label: string; color: string; icon: React.ReactNode }> = {
  pending:     { label: "Pending",     color: "text-muted-foreground", icon: <Circle      className="size-3.5 shrink-0" /> },
  ready:       { label: "Ready",       color: "text-blue-400",         icon: <ChevronRight className="size-3.5 shrink-0" /> },
  in_progress: { label: "In Progress", color: "text-yellow-400",       icon: <Loader2     className="size-3.5 shrink-0 animate-spin" /> },
  validating:  { label: "Validating",  color: "text-yellow-400",       icon: <Loader2     className="size-3.5 shrink-0 animate-spin" /> },
  completed:   { label: "Done",        color: "text-green-400",        icon: <CheckCircle2 className="size-3.5 shrink-0" /> },
  failed:      { label: "Failed",      color: "text-red-400",          icon: <AlertCircle  className="size-3.5 shrink-0" /> },
};

function goalStatusMeta(status: string) {
  return STATUS_META[status] ?? STATUS_META["pending"];
}

function formatTs(ts: number): string {
  const d = new Date(ts);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}:${String(d.getSeconds()).padStart(2, "0")}`;
}

const STAGE_BADGE_VARIANT: Record<string, "default" | "secondary" | "destructive" | "outline"> = {
  received: "outline",
  plan:     "secondary",
  tool:     "secondary",
  stream:   "outline",
  approval: "default",
  result:   "default",
  error:    "destructive",
  cancel:   "destructive",
};

// ─── sub-components ──────────────────────────────────────────────────────────

const PhaseStep: React.FC<{
  phase: Phase;
  active: Phase;
  completed: boolean;
  index: number;
}> = ({ phase, active, completed, index }) => {
  const isActive    = active === phase;
  const isPast      = completed || (
    (active === "execute" && phase === "scaffold") ||
    (active === "verify"  && phase !== "verify")
  );

  return (
    <div className={cn(
      "flex flex-col items-center gap-1.5 min-w-[80px]",
    )}>
      <div className={cn(
        "size-8 rounded-full flex items-center justify-center border transition-all duration-300",
        isActive  && "border-primary bg-primary/20 text-primary shadow-[0_0_12px_hsl(var(--color-primary)/0.4)]",
        isPast    && "border-green-500/60 bg-green-500/10 text-green-400",
        !isActive && !isPast && "border-border bg-card/60 text-muted-foreground",
      )}>
        {isPast && !isActive
          ? <CheckCircle2 className="size-4" />
          : <span className="text-xs font-bold">{index + 1}</span>
        }
      </div>
      <span className={cn(
        "text-[10px] font-semibold uppercase tracking-wide flex items-center gap-1",
        isActive  && "text-primary",
        isPast    && "text-green-400",
        !isActive && !isPast && "text-muted-foreground",
      )}>
        {PHASE_ICONS[phase]}
        {PHASE_LABELS[phase]}
      </span>
    </div>
  );
};

const GoalModuleRow: React.FC<{ goal: GoalNodeState; index: number }> = ({ goal, index }) => {
  const meta = goalStatusMeta(goal.status);
  return (
    <div className={cn(
      "flex items-start gap-3 p-3 rounded-lg border transition-colors",
      goal.status === "completed"   && "border-green-500/20 bg-green-500/5",
      goal.status === "in_progress" && "border-yellow-400/25 bg-yellow-400/5",
      goal.status === "failed"      && "border-red-500/20 bg-red-500/5",
      goal.status === "ready"       && "border-blue-400/20 bg-blue-400/5",
      goal.status === "pending"     && "border-border bg-card/40",
    )}>
      <div className={cn("mt-0.5 shrink-0", meta.color)}>{meta.icon}</div>
      <div className="flex-1 min-w-0">
        <p className="text-xs font-medium text-foreground leading-snug truncate">{goal.description}</p>
        <p className="text-[10px] text-muted-foreground mt-0.5 font-mono">{goal.id.slice(0, 8)}</p>
      </div>
      <Badge
        variant="outline"
        className={cn("text-[9px] shrink-0 uppercase tracking-wide", meta.color)}
      >
        {meta.label}
      </Badge>
    </div>
  );
};

const OrchestrationLog: React.FC<{ events: TimelineEventState[] }> = ({ events }) => {
  const endRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    endRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [events.length]);

  if (events.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center gap-2 py-10 text-muted-foreground">
        <TerminalSquare className="size-8 opacity-30" />
        <p className="text-xs">Waiting for execution events…</p>
      </div>
    );
  }

  return (
    <div className="space-y-1 font-mono text-[11px]">
      {events.map((ev) => (
        <div
          key={ev.id}
          className={cn(
            "flex items-start gap-3 px-2 py-1.5 rounded-md",
            ev.stage === "error"  && "bg-red-500/8 text-red-300",
            ev.stage === "result" && "bg-green-500/8 text-green-300",
            ev.stage === "tool"   && "bg-blue-500/8 text-blue-300",
            ev.stage === "plan"   && "bg-yellow-500/8 text-yellow-300",
            !["error","result","tool","plan"].includes(ev.stage) && "text-muted-foreground",
          )}
        >
          <span className="shrink-0 text-[10px] opacity-60 mt-0.5">{formatTs(ev.timestamp)}</span>
          <Badge
            variant={STAGE_BADGE_VARIANT[ev.stage] ?? "outline"}
            className="text-[9px] uppercase shrink-0 py-0"
          >
            {ev.stage}
          </Badge>
          <span className="min-w-0 truncate">{ev.title}</span>
        </div>
      ))}
      <div ref={endRef} />
    </div>
  );
};

const VerifyPanel: React.FC = () => {
  const qualityStatus = useStore((s) => s.qualityStatus);
  const vscode = useVSCodeAPI();

  const runHarness = () => {
    vscode.postMessage({ type: "chatMessage", text: "run_quality_harness" });
  };

  if (!qualityStatus) {
    return (
      <div className="flex flex-col items-center gap-4 py-12">
        <ShieldCheck className="size-12 text-muted-foreground/40" />
        <p className="text-sm text-muted-foreground">Quality harness not yet run.</p>
        <Button size="sm" onClick={runHarness} className="gap-2">
          <Play className="size-3.5" /> Run Quality Harness
        </Button>
      </div>
    );
  }

  const kpi = qualityStatus.latest?.kpi;

  return (
    <div className="space-y-4">
      <div className={cn(
        "flex items-center gap-3 p-4 rounded-xl border",
        qualityStatus.ok
          ? "border-green-500/30 bg-green-500/8"
          : "border-red-500/30 bg-red-500/8",
      )}>
        {qualityStatus.ok
          ? <CheckCircle2 className="size-6 text-green-400 shrink-0" />
          : <AlertCircle  className="size-6 text-red-400 shrink-0" />
        }
        <div>
          <p className="text-sm font-semibold">
            {qualityStatus.ok ? "All quality gates passed" : "Quality gates violated"}
          </p>
          {qualityStatus.message && (
            <p className="text-xs text-muted-foreground mt-0.5">{qualityStatus.message}</p>
          )}
        </div>
      </div>

      {kpi && (
        <div className="grid grid-cols-3 gap-3">
          {[
            { label: "Total Tests", value: kpi.total_tests ?? 0 },
            { label: "Passed",      value: kpi.passed ?? 0,     highlight: "text-green-400" },
            { label: "Failed",      value: kpi.failed ?? 0,     highlight: kpi.failed ? "text-red-400" : undefined },
          ].map(({ label, value, highlight }) => (
            <div key={label} className="rounded-xl border border-border bg-card/50 p-3 text-center">
              <p className={cn("text-2xl font-bold", highlight ?? "text-foreground")}>{value}</p>
              <p className="text-[10px] uppercase tracking-wide text-muted-foreground mt-1">{label}</p>
            </div>
          ))}
        </div>
      )}

      {kpi?.pass_rate !== undefined && (
        <div className="space-y-1.5">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Pass rate</span>
            <span className="font-semibold text-foreground">{(kpi.pass_rate * 100).toFixed(1)}%</span>
          </div>
          <div className="h-2 rounded-full bg-card border overflow-hidden">
            <div
              className={cn(
                "h-full rounded-full transition-all duration-700",
                kpi.pass_rate >= 0.9 ? "bg-green-500" : kpi.pass_rate >= 0.7 ? "bg-yellow-400" : "bg-red-500",
              )}
              style={{ width: `${(kpi.pass_rate * 100).toFixed(1)}%` }}
            />
          </div>
        </div>
      )}

      {qualityStatus.latest?.run_id && (
        <p className="text-[10px] font-mono text-muted-foreground">
          run_id: {qualityStatus.latest.run_id.slice(0, 16)}
          {qualityStatus.latest.duration_sec !== undefined &&
            ` · ${qualityStatus.latest.duration_sec.toFixed(1)}s`
          }
        </p>
      )}

      <Button size="sm" variant="outline" onClick={runHarness} className="w-full gap-2">
        <RefreshCw className="size-3.5" /> Re-run Quality Harness
      </Button>
    </div>
  );
};

// ─── main component ──────────────────────────────────────────────────────────

export const SplitAgentPanel: React.FC = () => {
  const vscode     = useVSCodeAPI();
  const connected  = useStore((s) => s.connected);
  const goals      = useStore((s) => s.goals);
  const alignment  = useStore((s) => s.alignment);
  const timeline   = useStore((s) => s.timeline);

  const [phase, setPhase]           = useState<Phase>("scaffold");
  const [initDesc, setInitDesc]     = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Derive computed values from real store data
  const completedGoals = useMemo(() => goals.filter((g) => g.status === "completed"), [goals]);
  const pendingGoals   = useMemo(() => goals.filter((g) => g.status === "pending" || g.status === "ready"), [goals]);
  const activeGoals    = useMemo(() => goals.filter((g) => g.status === "in_progress" || g.status === "validating"), [goals]);
  const failedGoals    = useMemo(() => goals.filter((g) => g.status === "failed"), [goals]);
  const completionPct  = goals.length > 0 ? completedGoals.length / goals.length : 0;

  // Auto-advance to verify when all goals done
  useEffect(() => {
    if (phase === "execute" && goals.length > 0 && pendingGoals.length === 0 && activeGoals.length === 0) {
      const t = setTimeout(() => setPhase("verify"), 800);
      return () => clearTimeout(t);
    }
  }, [phase, goals, pendingGoals, activeGoals]);

  // Refresh goals from MCP backend
  const refreshGoals = useCallback(() => {
    vscode.postMessage({ type: "chatMessage", text: "get_goal_graph" });
  }, [vscode]);

  // Initialise project
  const handleInit = useCallback(() => {
    if (!initDesc.trim()) return;
    setIsSubmitting(true);
    vscode.postMessage({ type: "chatMessage", text: `/init ${initDesc.trim()}` });
    setTimeout(() => {
      setIsSubmitting(false);
      setPhase("execute");
    }, 1500);
  }, [initDesc, vscode]);

  // Execute first pending goal
  const executeNext = useCallback(() => {
    const next = pendingGoals[0];
    if (!next) return;
    vscode.postMessage({
      type: "chatMessage",
      text: `Implement only the first pending goal: "${next.description}". No scaffolding. File changes + targeted tests only.`,
    });
  }, [pendingGoals, vscode]);

  // Move to verify phase manually
  const goVerify = useCallback(() => {
    setPhase("verify");
    vscode.postMessage({ type: "chatMessage", text: "run_quality_harness" });
  }, [vscode]);

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* ── Header ── */}
      <div className="shrink-0 px-6 pt-5 pb-4 space-y-4 border-b border-border">
        <div className="flex items-start justify-between gap-4">
          <div className="flex items-start gap-3">
            <div className="size-9 rounded-xl flex items-center justify-center bg-primary/15 border border-primary/25 shrink-0 mt-0.5">
              <Target className="size-4 text-primary" />
            </div>
            <div>
              <h2 className="text-sm font-bold tracking-tight">Split Agent Orchestration</h2>
              <p className="text-xs text-muted-foreground mt-0.5">
                Autonomous goal execution with alignment guardrails
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2 shrink-0">
            <div className={cn(
              "size-2 rounded-full",
              connected ? "bg-green-400 shadow-[0_0_6px_theme(colors.green.400)]" : "bg-muted",
            )} />
            <span className="text-[10px] text-muted-foreground uppercase tracking-wide">
              {connected ? "Connected" : "Offline"}
            </span>
          </div>
        </div>

        {/* Phase stepper */}
        <div className="flex items-center justify-between">
          {(["scaffold", "execute", "verify"] as Phase[]).map((p, i) => (
            <React.Fragment key={p}>
              <PhaseStep
                phase={p}
                active={phase}
                completed={false}
                index={i}
              />
              {i < 2 && (
                <div className={cn(
                  "flex-1 h-px mx-2 transition-colors duration-500",
                  (phase === "execute" && i === 0) || phase === "verify"
                    ? "bg-green-500/50" : "bg-border",
                )} />
              )}
            </React.Fragment>
          ))}
        </div>

        {/* Progress bar (goal completion) */}
        {goals.length > 0 && (
          <div className="space-y-1">
            <div className="flex justify-between text-[10px] text-muted-foreground">
              <span>{completedGoals.length}/{goals.length} goals complete</span>
              <span className="font-semibold text-foreground">{(completionPct * 100).toFixed(0)}%</span>
            </div>
            <div className="h-1.5 rounded-full bg-card border overflow-hidden">
              <div
                className="h-full rounded-full bg-gradient-to-r from-primary to-cyan-400 transition-all duration-700"
                style={{ width: `${(completionPct * 100).toFixed(1)}%` }}
              />
            </div>
          </div>
        )}
      </div>

      {/* ── Body ── */}
      <div className="flex-1 overflow-y-auto px-6 py-5 space-y-5">

        {/* ── PHASE 1: SCAFFOLD ── */}
        {phase === "scaffold" && (
          <>
            {goals.length === 0 ? (
              <div className="space-y-4">
                <div className="rounded-xl border border-dashed border-border bg-card/30 p-5 space-y-3 text-center">
                  <GitBranch className="size-10 mx-auto text-muted-foreground/40" />
                  <div>
                    <p className="text-sm font-medium">No project initialized</p>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      Describe your outcome and Sentinel will decompose it into atomic goals.
                    </p>
                  </div>
                  <div className="flex gap-2">
                    <input
                      type="text"
                      value={initDesc}
                      onChange={(e) => setInitDesc(e.target.value)}
                      onKeyDown={(e) => e.key === "Enter" && handleInit()}
                      placeholder="e.g. Build a REST API with JWT auth and PostgreSQL"
                      className="sentinel-input flex-1 text-xs"
                      disabled={!connected || isSubmitting}
                    />
                    <Button
                      size="sm"
                      onClick={handleInit}
                      disabled={!connected || !initDesc.trim() || isSubmitting}
                      className="shrink-0 gap-1.5"
                    >
                      {isSubmitting
                        ? <Loader2 className="size-3.5 animate-spin" />
                        : <Play    className="size-3.5" />
                      }
                      Init
                    </Button>
                  </div>
                </div>
              </div>
            ) : (
              <div className="space-y-4">
                <div className="flex items-center justify-between">
                  <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                    Generated Modules ({goals.length})
                  </p>
                  <div className="flex gap-2">
                    <Button size="sm" variant="ghost" onClick={refreshGoals} className="gap-1.5 h-7 text-xs">
                      <RefreshCw className="size-3" /> Refresh
                    </Button>
                    <Button
                      size="sm"
                      onClick={() => setPhase("execute")}
                      disabled={!connected}
                      className="gap-1.5 h-7 text-xs"
                    >
                      Proceed to Execute <ArrowRight className="size-3" />
                    </Button>
                  </div>
                </div>

                <div className="space-y-2">
                  {goals.map((g, i) => <GoalModuleRow key={g.id} goal={g} index={i} />)}
                </div>

                {alignment && (
                  <div className="flex items-center gap-2 p-2.5 rounded-lg border border-border bg-card/50">
                    <div className="size-2 rounded-full bg-primary shrink-0" />
                    <span className="text-xs text-muted-foreground">
                      Alignment: <strong className="text-foreground">{alignment.score.toFixed(1)}%</strong>
                      {" · "}{alignment.status}
                    </span>
                  </div>
                )}
              </div>
            )}
          </>
        )}

        {/* ── PHASE 2: EXECUTE ── */}
        {phase === "execute" && (
          <div className="space-y-4">
            {/* Live status strip */}
            <div className="grid grid-cols-3 gap-2">
              {[
                { label: "Active",    value: activeGoals.length,    color: "text-yellow-400" },
                { label: "Remaining", value: pendingGoals.length,   color: "text-muted-foreground" },
                { label: "Done",      value: completedGoals.length, color: "text-green-400" },
              ].map(({ label, value, color }) => (
                <div key={label} className="rounded-xl border border-border bg-card/50 px-3 py-2 text-center">
                  <p className={cn("text-xl font-bold", color)}>{value}</p>
                  <p className="text-[9px] uppercase tracking-wide text-muted-foreground mt-0.5">{label}</p>
                </div>
              ))}
            </div>

            {/* Pending goals queue */}
            {pendingGoals.length > 0 && (
              <div className="space-y-2">
                <div className="flex items-center justify-between">
                  <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
                    Execution Queue
                  </p>
                  <Button
                    size="sm"
                    onClick={executeNext}
                    disabled={!connected}
                    className="gap-1.5 h-7 text-xs"
                  >
                    <Play className="size-3.5" />
                    Execute Next
                  </Button>
                </div>
                <div className="space-y-2">
                  {pendingGoals.slice(0, 4).map((g, i) => (
                    <GoalModuleRow key={g.id} goal={g} index={i} />
                  ))}
                  {pendingGoals.length > 4 && (
                    <p className="text-xs text-muted-foreground text-center">
                      +{pendingGoals.length - 4} more pending
                    </p>
                  )}
                </div>
              </div>
            )}

            {/* Active goals */}
            {activeGoals.length > 0 && (
              <div className="space-y-2">
                <p className="text-xs font-semibold text-yellow-400/80 uppercase tracking-wide">
                  In Progress
                </p>
                {activeGoals.map((g, i) => <GoalModuleRow key={g.id} goal={g} index={i} />)}
              </div>
            )}

            {/* Failed goals */}
            {failedGoals.length > 0 && (
              <div className="space-y-2">
                <p className="text-xs font-semibold text-red-400/80 uppercase tracking-wide">
                  Failed — requires review
                </p>
                {failedGoals.map((g, i) => <GoalModuleRow key={g.id} goal={g} index={i} />)}
              </div>
            )}

            {/* Orchestration log */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <p className="text-xs font-semibold text-muted-foreground uppercase tracking-wide flex items-center gap-1.5">
                  <TerminalSquare className="size-3.5" /> Live Orchestration Log
                </p>
                <Badge variant="outline" className="text-[9px]">{timeline.length} events</Badge>
              </div>
              <div className="max-h-48 overflow-y-auto rounded-xl border border-border bg-card/30 p-3">
                <OrchestrationLog events={timeline.slice(-50)} />
              </div>
            </div>

            {/* Manual advance to verify */}
            <Button
              size="sm"
              variant="outline"
              onClick={goVerify}
              disabled={!connected}
              className="w-full gap-2"
            >
              <ShieldCheck className="size-3.5" /> Run Verification Now
            </Button>
          </div>
        )}

        {/* ── PHASE 3: VERIFY ── */}
        {phase === "verify" && (
          <div className="space-y-4">
            <VerifyPanel />

            {/* Summary of completed goals */}
            {completedGoals.length > 0 && (
              <div className="space-y-2">
                <p className="text-xs font-semibold text-green-400/80 uppercase tracking-wide">
                  Completed Goals
                </p>
                {completedGoals.map((g, i) => <GoalModuleRow key={g.id} goal={g} index={i} />)}
              </div>
            )}

            <Button
              size="sm"
              variant="outline"
              onClick={() => setPhase("execute")}
              className="w-full gap-2"
            >
              <ArrowRight className="size-3.5" /> Back to Execution
            </Button>
          </div>
        )}
      </div>

      {/* ── Footer ── */}
      {alignment && (
        <div className="shrink-0 border-t border-border px-6 py-2.5 flex items-center justify-between text-[10px] text-muted-foreground">
          <div className="flex items-center gap-3">
            <Clock className="size-3" />
            <span>Alignment: <strong className="text-foreground">{alignment.score.toFixed(1)}%</strong></span>
            <span className={cn(
              "flex items-center gap-0.5",
              alignment.trend > 0 && "text-green-400",
              alignment.trend < 0 && "text-red-400",
            )}>
              {alignment.trend > 0 ? "↑" : alignment.trend < 0 ? "↓" : "→"} {alignment.status}
            </span>
          </div>
          <span className="font-mono opacity-60">{(completionPct * 100).toFixed(0)}% complete</span>
        </div>
      )}
    </div>
  );
};

export default SplitAgentPanel;
