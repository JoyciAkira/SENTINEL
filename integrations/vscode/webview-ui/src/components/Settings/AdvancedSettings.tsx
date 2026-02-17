/**
 * AdvancedSettings — Configurazione avanzata reale.
 *
 * Sezioni:
 * 1. Memory Management  — export/import/clear session memory via MCP
 * 2. Reliability SLO    — visualizzazione soglie live dallo store
 * 3. Governance         — mostra policy attive e proposta pending
 * 4. Runtime Info       — tool count, server version, capabilities
 * 5. Calibrazione       — sensitivity slider per alignment threshold
 */
import React, { useCallback, useState } from "react";
import {
  Database,
  Download,
  Upload,
  Trash2,
  ShieldCheck,
  Activity,
  Cpu,
  SlidersHorizontal,
  ChevronDown,
  ChevronRight,
  CheckCircle2,
  AlertCircle,
  ExternalLink,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";

// ─── Accordion section ────────────────────────────────────────────────────────

const Section: React.FC<{
  icon: React.ReactNode;
  title: string;
  badge?: string;
  badgeVariant?: "default" | "secondary" | "destructive" | "outline";
  defaultOpen?: boolean;
  children: React.ReactNode;
}> = ({ icon, title, badge, badgeVariant = "outline", defaultOpen = false, children }) => {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="rounded-xl border border-border overflow-hidden">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center gap-3 px-4 py-3 bg-card/60 hover:bg-card/80 transition-colors text-left"
      >
        <span className="text-muted-foreground shrink-0">{icon}</span>
        <span className="text-sm font-semibold flex-1">{title}</span>
        {badge && (
          <Badge variant={badgeVariant} className="text-[9px] uppercase shrink-0 mr-1">
            {badge}
          </Badge>
        )}
        {open
          ? <ChevronDown  className="size-3.5 text-muted-foreground shrink-0" />
          : <ChevronRight className="size-3.5 text-muted-foreground shrink-0" />
        }
      </button>
      {open && (
        <div className="px-4 py-4 bg-card/30 border-t border-border space-y-3">
          {children}
        </div>
      )}
    </div>
  );
};

// ─── Row helper ───────────────────────────────────────────────────────────────

const KVRow: React.FC<{ label: string; value: React.ReactNode }> = ({ label, value }) => (
  <div className="flex items-center justify-between gap-4 text-xs">
    <span className="text-muted-foreground shrink-0">{label}</span>
    <span className="font-medium text-foreground text-right font-mono break-all">{value}</span>
  </div>
);

// ─── Main component ───────────────────────────────────────────────────────────

export const AdvancedSettings: React.FC = () => {
  const vscode              = useVSCodeAPI();
  const reliability         = useStore((s) => s.reliability);
  const reliabilityThresh   = useStore((s) => s.reliabilityThresholds);
  const reliabilitySlo      = useStore((s) => s.reliabilitySlo);
  const governance          = useStore((s) => s.governance);
  const runtimeCapabilities = useStore((s) => s.runtimeCapabilities);
  const augmentSettings     = useStore((s) => s.augmentSettings);
  const setAugmentSettings  = useStore((s) => s.setAugmentSettings);

  const [sensitivity, setSensitivity]     = useState(75);
  const [exportStatus, setExportStatus]   = useState<"idle" | "busy" | "done">("idle");
  const [clearStatus,  setClearStatus]    = useState<"idle" | "busy" | "done">("idle");

  // ── memory management ──
  const handleExportMemory = useCallback(() => {
    setExportStatus("busy");
    vscode.postMessage({ type: "chatMessage", text: "export_memory" });
    setTimeout(() => setExportStatus("done"), 2000);
    setTimeout(() => setExportStatus("idle"), 4000);
  }, [vscode]);

  const handleClearMemory = useCallback(() => {
    if (!window.confirm("Clear all session memory? This cannot be undone.")) return;
    setClearStatus("busy");
    vscode.postMessage({ type: "chatMessage", text: "clear_session_memory" });
    setTimeout(() => setClearStatus("done"), 1500);
    setTimeout(() => setClearStatus("idle"), 3000);
  }, [vscode]);

  const handleImportMemory = useCallback(() => {
    vscode.postMessage({ type: "chatMessage", text: "import_memory" });
  }, [vscode]);

  // ── calibration ──
  const applySensitivity = useCallback(() => {
    vscode.postMessage({ type: "chatMessage", text: `calibrate_alignment_threshold ${sensitivity}` });
  }, [vscode, sensitivity]);

  // ── governance ──
  const approveProposal = useCallback((id: string) => {
    vscode.postMessage({ type: "chatMessage", text: `approve_governance_proposal ${id}` });
  }, [vscode]);

  const rejectProposal = useCallback((id: string) => {
    vscode.postMessage({ type: "chatMessage", text: `reject_governance_proposal ${id}` });
  }, [vscode]);

  return (
    <div className="space-y-3 p-4 max-w-2xl">
      {/* ── 1. Memory Management ── */}
      <Section
        icon={<Database className="size-4" />}
        title="Memory Management"
        defaultOpen
      >
        <p className="text-xs text-muted-foreground">
          Session memory holds the pinned transcript, goal history and tool call ledger.
          Export before clearing to preserve audit trail.
        </p>
        <div className="flex flex-wrap gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={handleExportMemory}
            disabled={exportStatus === "busy"}
            className="gap-1.5"
          >
            {exportStatus === "busy"
              ? <Activity className="size-3.5 animate-pulse" />
              : exportStatus === "done"
              ? <CheckCircle2 className="size-3.5 text-green-400" />
              : <Download className="size-3.5" />
            }
            {exportStatus === "done" ? "Exported!" : "Export Memory"}
          </Button>
          <Button size="sm" variant="outline" onClick={handleImportMemory} className="gap-1.5">
            <Upload  className="size-3.5" /> Import
          </Button>
          <Button
            size="sm"
            variant="destructive"
            onClick={handleClearMemory}
            disabled={clearStatus === "busy"}
            className="gap-1.5 ml-auto"
          >
            {clearStatus === "busy"
              ? <Activity className="size-3.5 animate-pulse" />
              : clearStatus === "done"
              ? <CheckCircle2 className="size-3.5" />
              : <Trash2 className="size-3.5" />
            }
            {clearStatus === "done" ? "Cleared" : "Clear Session"}
          </Button>
        </div>
      </Section>

      {/* ── 2. Reliability SLO ── */}
      <Section
        icon={<Activity className="size-4" />}
        title="Reliability SLO"
        badge={reliabilitySlo ? (reliabilitySlo.healthy ? "healthy" : "violated") : "no data"}
        badgeVariant={reliabilitySlo?.healthy ? "default" : reliabilitySlo ? "destructive" : "outline"}
      >
        {reliability && reliabilityThresh ? (
          <div className="space-y-2">
            {[
              {
                label: "Task success rate",
                value: reliability.task_success_rate,
                threshold: reliabilityThresh.min_task_success_rate,
                invert: false,
              },
              {
                label: "No-regression rate",
                value: reliability.no_regression_rate,
                threshold: reliabilityThresh.min_no_regression_rate,
                invert: false,
              },
              {
                label: "Rollback rate",
                value: reliability.rollback_rate,
                threshold: reliabilityThresh.max_rollback_rate,
                invert: true,
              },
              {
                label: "Invariant violation rate",
                value: reliability.invariant_violation_rate,
                threshold: reliabilityThresh.max_invariant_violation_rate,
                invert: true,
              },
            ].map(({ label, value, threshold, invert }) => {
              const pct  = Math.min(value * 100, 100);
              const ok   = invert ? value <= threshold : value >= threshold;
              return (
                <div key={label} className="space-y-1">
                  <div className="flex justify-between text-[10px] text-muted-foreground">
                    <span>{label}</span>
                    <span className={cn("font-semibold", ok ? "text-green-400" : "text-red-400")}>
                      {(value * 100).toFixed(1)}% / {(threshold * 100).toFixed(0)}% min
                    </span>
                  </div>
                  <div className="h-1.5 rounded-full bg-card border overflow-hidden">
                    <div
                      className={cn("h-full rounded-full transition-all", ok ? "bg-green-500" : "bg-red-500")}
                      style={{ width: `${pct}%` }}
                    />
                  </div>
                </div>
              );
            })}

            <KVRow
              label="Avg time to recover"
              value={`${(reliability.avg_time_to_recover_ms / 1000).toFixed(2)}s`}
            />

            {reliabilitySlo?.violations.length ? (
              <div className="rounded-lg border border-red-500/30 bg-red-500/8 p-3 space-y-1">
                <p className="text-[10px] font-semibold text-red-400 uppercase tracking-wide">
                  Active violations
                </p>
                {reliabilitySlo.violations.map((v) => (
                  <p key={v} className="text-xs text-red-300">{v}</p>
                ))}
              </div>
            ) : null}
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">
            No reliability data yet. Start an agent session to collect metrics.
          </p>
        )}
      </Section>

      {/* ── 3. Governance ── */}
      <Section
        icon={<ShieldCheck className="size-4" />}
        title="Governance & Policy"
        badge={governance?.pending_proposal ? "proposal pending" : undefined}
        badgeVariant="destructive"
      >
        {governance ? (
          <div className="space-y-3">
            {/* Pending proposal */}
            {governance.pending_proposal && (
              <div className="rounded-xl border border-yellow-500/30 bg-yellow-500/8 p-3 space-y-2">
                <div className="flex items-center justify-between">
                  <p className="text-xs font-semibold text-yellow-400">Pending Proposal</p>
                  <Badge variant="outline" className="text-[9px]">
                    {governance.pending_proposal.status}
                  </Badge>
                </div>
                <p className="text-xs text-muted-foreground">{governance.pending_proposal.rationale}</p>
                <p className="text-[10px] font-mono text-muted-foreground">
                  id: {governance.pending_proposal.id.slice(0, 16)}…
                </p>
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    className="gap-1.5"
                    onClick={() => approveProposal(governance.pending_proposal!.id)}
                  >
                    <CheckCircle2 className="size-3.5" /> Approve
                  </Button>
                  <Button
                    size="sm"
                    variant="destructive"
                    className="gap-1.5"
                    onClick={() => rejectProposal(governance.pending_proposal!.id)}
                  >
                    <AlertCircle className="size-3.5" /> Reject
                  </Button>
                </div>
              </div>
            )}

            {/* Policy summary */}
            <div className="space-y-1.5">
              <KVRow label="Required frameworks"  value={governance.required_frameworks.join(", ") || "—"} />
              <KVRow label="Allowed frameworks"   value={governance.allowed_frameworks.join(", ")  || "any"} />
              <KVRow label="Allowed ports"        value={governance.allowed_ports.join(", ")         || "any"} />
              <KVRow label="History size"         value={String(governance.history_size)} />
              {governance.world_model?.how_enforced?.manifold_version !== undefined && (
                <KVRow
                  label="Manifold version"
                  value={`v${governance.world_model.how_enforced.manifold_version}`}
                />
              )}
            </div>
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">No governance data received yet.</p>
        )}
      </Section>

      {/* ── 4. Calibration ── */}
      <Section icon={<SlidersHorizontal className="size-4" />} title="Alignment Calibration">
        <p className="text-xs text-muted-foreground">
          Sensitivity controls how aggressively drift is penalised. Lower = more tolerant.
          Higher = stricter alignment enforcement.
        </p>
        <div className="space-y-2">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>Sensitivity threshold</span>
            <span className="font-semibold text-foreground">{sensitivity}%</span>
          </div>
          <input
            type="range"
            min={10}
            max={100}
            step={5}
            value={sensitivity}
            onChange={(e) => setSensitivity(Number(e.target.value))}
            className="w-full accent-primary cursor-pointer"
          />
          <div className="flex justify-between text-[10px] text-muted-foreground">
            <span>Lenient (10%)</span>
            <span>Strict (100%)</span>
          </div>
          <Button size="sm" variant="outline" onClick={applySensitivity} className="gap-1.5">
            <SlidersHorizontal className="size-3.5" /> Apply Calibration
          </Button>
        </div>
      </Section>

      {/* ── 5. Runtime Info ── */}
      <Section icon={<Cpu className="size-4" />} title="Runtime Capabilities">
        {runtimeCapabilities ? (
          <div className="space-y-2">
            <KVRow label="Server"    value={runtimeCapabilities.server_name} />
            <KVRow label="Version"   value={runtimeCapabilities.server_version} />
            <KVRow label="Tools"     value={String(runtimeCapabilities.tool_count)} />
            <KVRow
              label="Status"
              value={
                <span className={runtimeCapabilities.connected ? "text-green-400" : "text-red-400"}>
                  {runtimeCapabilities.connected ? "Connected" : "Disconnected"}
                </span>
              }
            />
            {runtimeCapabilities.tools.length > 0 && (
              <div className="pt-1">
                <p className="text-[10px] text-muted-foreground uppercase tracking-wide mb-2">
                  Registered Tools
                </p>
                <div className="flex flex-wrap gap-1.5">
                  {runtimeCapabilities.tools.map((t) => (
                    <span
                      key={t}
                      className="border border-border rounded-md px-1.5 py-0.5 text-[10px] font-mono text-muted-foreground bg-card/50"
                    >
                      {t}
                    </span>
                  ))}
                </div>
              </div>
            )}
          </div>
        ) : (
          <p className="text-xs text-muted-foreground">
            No runtime data. Connect to an MCP server to populate this section.
          </p>
        )}
      </Section>

      {/* ── 6. Augment Mode ── */}
      <Section icon={<ExternalLink className="size-4" />} title="Augment Mode">
        <div className="space-y-3">
          <KVRow label="Mode"       value={augmentSettings.mode} />
          <KVRow label="Enabled"    value={augmentSettings.enabled ? "Yes" : "No"} />
          <KVRow label="Enforce BYO" value={augmentSettings.enforceByo ? "Yes" : "No"} />
          <div className="flex gap-2 flex-wrap">
            {(["disabled", "internal_only", "byo_customer"] as const).map((mode) => (
              <Button
                key={mode}
                size="sm"
                variant={augmentSettings.mode === mode ? "default" : "outline"}
                className="text-[10px] h-7"
                onClick={() => {
                  setAugmentSettings({ ...augmentSettings, mode });
                  vscode.postMessage({ type: "chatMessage", text: `set_augment_mode ${mode}` });
                }}
              >
                {mode}
              </Button>
            ))}
          </div>
        </div>
      </Section>
    </div>
  );
};

export default AdvancedSettings;
