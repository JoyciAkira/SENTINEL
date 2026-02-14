import React, { useMemo } from "react";
import { useStore } from "../../state/store";
import type { FileOperation, ToolCallInfo } from "../../state/types";

function formatStatus(status: ToolCallInfo["status"]) {
  if (status === "success") return "Safe";
  if (status === "error") return "Risk";
  return "Pending";
}

export default function ActionPreview() {
  const messages = useStore((s) => s.messages);

  const { toolCalls, fileOps } = useMemo(() => {
    const tools: ToolCallInfo[] = [];
    const files: FileOperation[] = [];

    // Prendi le ultime operazioni più rilevanti
    for (let i = messages.length - 1; i >= 0; i--) {
      const msg = messages[i];
      if (msg.toolCalls) tools.push(...msg.toolCalls);
      if (msg.fileOperations) files.push(...msg.fileOperations);
      if (tools.length >= 5 && files.length >= 5) break;
    }

    return {
      toolCalls: tools.slice(0, 5),
      fileOps: files.slice(0, 5),
    };
  }, [messages]);

  return (
    <div className="preview-body">
      <div className="preview-section">
        <div className="section-header">
          <span className="preview-section__title">Pending Logic Gate</span>
          <span className="mono" style={{ fontSize: "10px" }}>{toolCalls.length} calls</span>
        </div>
        {toolCalls.length === 0 ? (
          <div className="empty-state" style={{ padding: "20px", border: "1px dashed var(--border)", borderRadius: "var(--radius-md)" }}>
            Awaiting tool interactions...
          </div>
        ) : (
          <div className="preview-list">
            {toolCalls.map((tool, index) => (
              <div
                key={`${tool.name}-${index}`}
                className="preview-item"
                style={{ 
                  animation: `fadeIn 0.4s ease forwards`,
                  animationDelay: `${index * 60}ms`,
                  opacity: 0,
                  transform: "translateY(10px)"
                }}
              >
                <div style={{ display: "flex", alignItems: "center", gap: "10px" }}>
                  <div style={{ width: "8px", height: "8px", borderRadius: "50%", background: tool.status === "success" ? "var(--accent)" : "var(--warning)" }} />
                  <div>
                    <div className="preview-item__title" style={{ fontSize: "12px", fontFamily: "var(--font-mono)" }}>{tool.name}</div>
                    <div className="preview-item__meta">{Object.keys(tool.arguments ?? {}).length} parameters</div>
                  </div>
                </div>
                <span className={`pill ${tool.status === "success" ? "pill--success" : "pill--pending"}`} style={{ fontSize: "10px", fontWeight: 600 }}>
                  {formatStatus(tool.status)}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="preview-section" style={{ marginTop: "12px" }}>
        <div className="section-header">
          <span className="preview-section__title">Proposed File Changes</span>
          <span className="mono" style={{ fontSize: "10px" }}>{fileOps.length} files</span>
        </div>
        {fileOps.length === 0 ? (
          <div className="empty-state" style={{ padding: "20px", border: "1px dashed var(--border)", borderRadius: "var(--radius-md)" }}>
            No modifications proposed yet.
          </div>
        ) : (
          <div className="preview-list">
            {fileOps.map((file, index) => (
              <div
                key={`${file.path}-${index}`}
                className="preview-item"
                style={{ 
                  animation: `fadeIn 0.4s ease forwards`,
                  animationDelay: `${(index + toolCalls.length) * 60}ms`,
                  opacity: 0,
                  transform: "translateY(10px)"
                }}
              >
                <div>
                  <div className="preview-item__title">{file.path.split('/').pop()}</div>
                  <div className="preview-item__meta" style={{ fontFamily: "var(--font-mono)" }}>
                    {file.type.toUpperCase()} <span style={{ color: "var(--accent)" }}>+{file.linesAdded || 0}</span> <span style={{ color: "var(--danger)" }}>-{file.linesRemoved || 0}</span>
                  </div>
                </div>
                <div style={{ display: "flex", gap: "6px" }}>
                   <button className="btn" style={{ padding: "4px 10px", fontSize: "10px", background: file.approved ? "var(--bg-surface-3)" : "var(--accent)", color: file.approved ? "var(--text-muted)" : "white" }}>
                    {file.approved ? "Diff" : "Approve"}
                   </button>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      <style>{`
        @keyframes fadeIn {
          to {
            opacity: 1;
            transform: translateY(0);
          }
        }
      `}</style>
    </div>
  );
}
