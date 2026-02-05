import React, { useEffect, useRef, useState } from "react";
import { useStore } from "../../state/store";

export default function AlignmentGauge() {
  const alignment = useStore((s) => s.alignment);
  const [pulse, setPulse] = useState(false);
  const prevScoreRef = useRef<number | null>(null);
  const score = alignment?.score ?? null;

  useEffect(() => {
    if (score === null) {
      prevScoreRef.current = null;
      return;
    }
    const prev = prevScoreRef.current;
    prevScoreRef.current = score;
    if (prev !== null && prev !== score) {
      setPulse(true);
      const timeout = window.setTimeout(() => setPulse(false), 600);
      return () => window.clearTimeout(timeout);
    }
    return undefined;
  }, [score]);

  if (!alignment) {
    return (
      <div className="alignment">
        <div className="empty-state">No alignment data.</div>
      </div>
    );
  }

  const scoreValue = score ?? 0;
  const barColor = scoreValue >= 85 
    ? "var(--accent)" 
    : scoreValue >= 60 
      ? "var(--warning)" 
      : "var(--danger)";

  const glowStyle = scoreValue >= 85 ? {
    boxShadow: `0 0 15px rgba(20, 184, 166, 0.4)`
  } : {};

  return (
    <div className={`alignment${pulse ? " alignment--pulse" : ""}`}>
      <div className="card-header" style={{ marginBottom: "12px", padding: 0 }}>
        <div>
          <h2 style={{ fontSize: "14px", fontWeight: 600 }}>{alignment.status}</h2>
          <span style={{ fontSize: "11px" }}>Predictive integrity score</span>
        </div>
        <div className="chip" style={{ background: barColor, color: "white", border: "none" }}>
          {scoreValue.toFixed(1)}%
        </div>
      </div>
      
      <div className="meter" style={{ height: "12px", background: "var(--bg-surface-3)" }}>
        <span
          style={{
            width: `${Math.min(100, Math.max(0, scoreValue))}%`,
            background: `linear-gradient(90deg, var(--accent), var(--accent-2))`,
            transition: "width 1s cubic-bezier(0.4, 0, 0.2, 1)",
            ...glowStyle
          }}
        />
      </div>

      <div className="mini-grid" style={{ marginTop: "16px" }}>
        <div className="insight">
          <strong>Trend</strong>
          <span style={{ color: alignment.trend >= 0 ? "var(--accent)" : "var(--danger)" }}>
            {alignment.trend >= 0 ? "↑" : "↓"} {Math.abs(alignment.trend).toFixed(1)}%
          </span>
        </div>
        <div className="insight">
          <strong>Confidence</strong>
          <span>{(alignment.confidence * 100).toFixed(0)}%</span>
        </div>
        <div className="insight">
          <strong>Volatility</strong>
          <span>Low</span>
        </div>
        <div className="insight">
          <strong>Drift Risk</strong>
          <span style={{ color: scoreValue < 70 ? "var(--warning)" : "inherit" }}>
            {Math.max(0, 100 - scoreValue).toFixed(0)}%
          </span>
        </div>
      </div>
    </div>
  );
}
