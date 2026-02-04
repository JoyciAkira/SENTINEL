import React, { useState } from "react";
import type { ToolCallInfo } from "../../state/types";

interface Props {
  tool: ToolCallInfo;
}

const STATUS_LABELS: Record<string, string> = {
  pending: "\u23F3 Running...",
  success: "\u2705",
  error: "\u274C Error",
};

export default function ToolCallCard({ tool }: Props) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="tool-card tool-card--enter">
      <div onClick={() => setExpanded(!expanded)} className="tool-card__header">
        <span>
          <strong>Tool:</strong> {tool.name}
        </span>
        <span className={`tool-card__status tool-card__status--${tool.status}`}>
          {STATUS_LABELS[tool.status] ?? tool.status}
        </span>
      </div>
      {expanded && (
        <div className="tool-card__body">
          <div style={{ marginBottom: "4px" }}>
            <strong>Arguments:</strong>
            <pre
              style={{
                margin: "4px 0",
                whiteSpace: "pre-wrap",
                wordBreak: "break-word",
              }}
            >
              {JSON.stringify(tool.arguments, null, 2)}
            </pre>
          </div>
          {tool.result && (
            <div>
              <strong>Result:</strong>
              <pre
                style={{
                  margin: "4px 0",
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-word",
                }}
              >
                {tool.result}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
