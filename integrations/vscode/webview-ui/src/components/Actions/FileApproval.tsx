import React, { useMemo, useState } from "react";
import type { FileOperation } from "../../state/types";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";

interface Props {
  operation: FileOperation;
  messageId: string;
}

export default function FileApproval({ operation, messageId }: Props) {
  const vscode = useVSCodeAPI();
  const [showDiff, setShowDiff] = useState(false);
  const diffLines = useMemo(() => {
    if (!operation.diff) return [];
    return operation.diff.split("\n");
  }, [operation.diff]);
  const previewLines = diffLines.slice(0, 10);

  const handleApprove = () => {
    vscode.postMessage({
      type: "fileApproval",
      messageId,
      path: operation.path,
      approved: true,
    });
  };

  const handleReject = () => {
    vscode.postMessage({
      type: "fileApproval",
      messageId,
      path: operation.path,
      approved: false,
    });
  };

  if (operation.approved !== undefined) {
    return (
      <div className="file-approval">
        <span>
          {operation.approved ? "\u2705" : "\u274C"} {operation.path}
        </span>
      </div>
    );
  }

  return (
    <div className="file-approval">
      <div className="file-approval__header">
        <span className="file-approval__title">File: {operation.path}</span>
        <span className="file-approval__meta">
          {operation.type === "create" ? "NEW" : operation.type.toUpperCase()}
          {operation.linesAdded ? ` +${operation.linesAdded}` : ""}
          {operation.linesRemoved ? ` -${operation.linesRemoved}` : ""}
        </span>
      </div>
      {operation.diff && (
        <button
          type="button"
          className="file-approval__diff-toggle"
          onClick={() => setShowDiff((prev) => !prev)}
        >
          {showDiff ? "Hide diff" : "Show diff"}
        </button>
      )}
      {showDiff && operation.diff && (
        <div className="diff-snippet">
          {previewLines.map((line, idx) => {
            const kind = line.startsWith("+")
              ? "diff-line--add"
              : line.startsWith("-")
                ? "diff-line--del"
                : "diff-line--meta";
            return (
              <div key={`${idx}-${line}`} className={`diff-line ${kind}`}>
                {line === "" ? " " : line}
              </div>
            );
          })}
          {diffLines.length > previewLines.length && (
            <div className="diff-line diff-line--meta">…</div>
          )}
        </div>
      )}
      <div className="file-approval__actions">
        <button onClick={handleApprove} className="btn btn--ghost">
          Approve
        </button>
        <button onClick={handleReject} className="btn btn--subtle">
          Reject
        </button>
      </div>
    </div>
  );
}
