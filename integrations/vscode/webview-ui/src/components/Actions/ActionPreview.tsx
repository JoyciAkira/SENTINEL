import React, { useMemo } from "react";
import { useStore } from "../../state/store";
import type { FileOperation, ToolCallInfo } from "../../state/types";

function formatStatus(status: ToolCallInfo["status"]) {
  if (status === "success") return "Success";
  if (status === "error") return "Error";
  return "Pending";
}

export default function ActionPreview() {
  const messages = useStore((s) => s.messages);

  const { toolCalls, fileOps } = useMemo(() => {
    const tools: ToolCallInfo[] = [];
    const files: FileOperation[] = [];

    for (let i = messages.length - 1; i >= 0; i--) {
      const msg = messages[i];
      if (msg.toolCalls) {
        tools.push(...msg.toolCalls);
      }
      if (msg.fileOperations) {
        files.push(...msg.fileOperations);
      }
      if (tools.length >= 6 && files.length >= 6) break;
    }

    return {
      toolCalls: tools.slice(0, 6),
      fileOps: files.slice(0, 6),
    };
  }, [messages]);

  return (
    <div className="preview-body">
      <div className="preview-section">
        <div className="preview-section__title">Tool Calls</div>
        {toolCalls.length === 0 ? (
          <div className="empty-state">No tool calls yet.</div>
        ) : (
          <div className="preview-list">
            {toolCalls.map((tool, index) => (
              <div
                key={`${tool.name}-${index}`}
                className="preview-item"
                style={{ animationDelay: `${Math.min(index * 40, 200)}ms` }}
              >
                <div>
                  <div className="preview-item__title">{tool.name}</div>
                  <div className="preview-item__meta">
                    {Object.keys(tool.arguments ?? {}).length} args
                  </div>
                </div>
                <span className={`pill pill--${tool.status}`}>
                  {formatStatus(tool.status)}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      <div className="preview-section">
        <div className="preview-section__title">File Operations</div>
        {fileOps.length === 0 ? (
          <div className="empty-state">No file changes yet.</div>
        ) : (
          <div className="preview-list">
            {fileOps.map((file, index) => (
              <div
                key={`${file.path}-${index}`}
                className="preview-item"
                style={{ animationDelay: `${Math.min(index * 40, 200)}ms` }}
              >
                <div>
                  <div className="preview-item__title">{file.path}</div>
                  <div className="preview-item__meta">
                    {file.type.toUpperCase()}{" "}
                    {file.linesAdded ? `+${file.linesAdded}` : ""}{" "}
                    {file.linesRemoved ? `-${file.linesRemoved}` : ""}
                  </div>
                  {file.diff && (
                    <div className="preview-item__diff">
                      {file.diff
                        .split("\n")
                        .find(
                          (line) =>
                            line.startsWith("+") || line.startsWith("-"),
                        )}
                    </div>
                  )}
                </div>
                <span
                  className={`pill ${file.approved ? "pill--success" : "pill--pending"}`}
                >
                  {file.approved ? "Approved" : "Review"}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
