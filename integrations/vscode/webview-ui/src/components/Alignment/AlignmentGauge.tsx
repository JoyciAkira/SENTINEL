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
        <span className="chat-subtitle">Alignment: --</span>
      </div>
    );
  }
  const trendArrow =
    alignment.trend > 0 ? "\u25B2" : alignment.trend < 0 ? "\u25BC" : "";
  const trendText =
    alignment.trend !== 0
      ? ` ${trendArrow} ${Math.abs(alignment.trend).toFixed(1)}`
      : "";

  const barColor =
    score >= 75 ? "#4caf50" : score >= 40 ? "#ff9800" : "#f44336";
  const pct = Math.min(100, Math.max(0, score));

  return (
    <div className={`alignment${pulse ? " alignment--pulse" : ""}`}>
      <div className="section-header" style={{ marginBottom: "6px" }}>
        <span className="section-title">
          Alignment {score.toFixed(1)}%{trendText}
        </span>
        <span className="chat-subtitle">{alignment.status}</span>
      </div>
      <div className="alignment-bar">
        <div
          className="alignment-bar__fill"
          style={{ width: `${pct}%`, backgroundColor: barColor }}
        />
      </div>
    </div>
  );
}
