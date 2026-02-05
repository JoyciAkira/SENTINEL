import React, { useMemo, useState } from "react";
import type { FileOperation } from "../../state/types";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";
import { cn } from "@/lib/utils";
import { FileCode, Check, X, ChevronDown, ChevronUp, Diff } from "lucide-react";
import { Button } from "../ui/button";
import { Badge } from "../ui/badge";

export default function FileApproval({ operation, messageId }: { operation: FileOperation; messageId: string }) {
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
      <div className={cn(
        "flex items-center gap-3 px-3 py-2 rounded-lg border bg-accent/20",
        operation.approved ? "border-primary/20" : "border-destructive/20"
      )}>
        <div className={cn(
          "p-1 rounded-md",
          operation.approved ? "bg-primary/10 text-primary" : "bg-destructive/10 text-destructive"
        )}>
          {operation.approved ? <Check className="size-3" /> : <X className="size-3" />}
        </div>
        <span className="text-xs font-medium truncate flex-1">{operation.path}</span>
        <Badge variant="outline" className="text-[10px]">
          {operation.approved ? 'APPROVED' : 'REJECTED'}
        </Badge>
      </div>
    );
  }

  return (
    <div className="rounded-lg border bg-card overflow-hidden shadow-sm border-l-4 border-l-primary">
      <div className="px-3 py-2 border-b bg-accent/10 flex items-center justify-between">
        <div className="flex items-center gap-2 overflow-hidden">
          <FileCode className="size-4 text-primary shrink-0" />
          <span className="text-xs font-semibold truncate">{operation.path}</span>
        </div>
        <Badge variant="secondary" className="text-[10px]">
          {operation.type.toUpperCase()}
        </Badge>
      </div>
      
      <div className="p-3 space-y-3">
        <div className="flex items-center justify-between text-[11px]">
          <div className="flex gap-3">
            {operation.linesAdded !== undefined && (
              <span className="text-primary font-bold">+{operation.linesAdded} lines</span>
            )}
            {operation.linesRemoved !== undefined && (
              <span className="text-destructive font-bold">-{operation.linesRemoved} lines</span>
            )}
          </div>
          {operation.diff && (
            <Button 
              variant="ghost" 
              size="xs" 
              onClick={() => setShowDiff(!showDiff)}
              className="h-6 gap-1"
            >
              <Diff className="size-3" />
              {showDiff ? 'Hide Diff' : 'View Diff'}
            </Button>
          )}
        </div>

        {showDiff && operation.diff && (
          <div className="rounded border bg-muted font-mono text-[11px] overflow-hidden animate-in slide-in-from-top-2">
            <div className="p-2 space-y-0.5 max-h-48 overflow-y-auto">
              {previewLines.map((line, idx) => {
                const isAdd = line.startsWith("+");
                const isDel = line.startsWith("-");
                return (
                  <div key={idx} className={cn(
                    "whitespace-pre-wrap break-all px-1 rounded",
                    isAdd ? "bg-primary/10 text-primary" : isDel ? "bg-destructive/10 text-destructive" : "opacity-60"
                  )}>
                    {line || " "}
                  </div>
                );
              })}
              {diffLines.length > previewLines.length && (
                <div className="opacity-40 px-1 italic">... {diffLines.length - previewLines.length} more lines</div>
              )}
            </div>
          </div>
        )}

        <div className="flex gap-2">
          <Button onClick={handleApprove} size="sm" className="flex-1 h-8 gap-1.5">
            <Check className="size-3" /> Approve
          </Button>
          <Button onClick={handleReject} variant="outline" size="sm" className="flex-1 h-8 gap-1.5 hover:bg-destructive/10 hover:text-destructive">
            <X className="size-3" /> Reject
          </Button>
        </div>
      </div>
    </div>
  );
}
