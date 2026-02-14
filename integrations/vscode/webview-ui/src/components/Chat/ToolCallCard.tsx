import React, { useState } from "react";
import type { ToolCallInfo } from "../../state/types";
import { cn } from "@/lib/utils";
import { ChevronDown, ChevronUp, Terminal, CheckCircle2, XCircle, Loader2 } from "lucide-react";
import { Button } from "../ui/button";

export default function ToolCallCard({ tool }: { tool: ToolCallInfo }) {
  const [expanded, setExpanded] = useState(false);

  const isSuccess = tool.status === "success";
  const isError = tool.status === "error";
  const isPending = tool.status === "pending";

  return (
    <div className={cn(
      "rounded-lg border bg-card overflow-hidden shadow-sm transition-all border-l-4",
      isSuccess ? "border-l-primary" : isError ? "border-l-destructive" : "border-l-orange-500"
    )}>
      <div 
        onClick={() => setExpanded(!expanded)} 
        className="flex items-center justify-between px-3 py-2 cursor-pointer hover:bg-accent/50 transition-colors"
      >
        <div className="flex items-center gap-2">
          <div className={cn(
            "p-1 rounded-md",
            isSuccess ? "bg-primary/10 text-primary" : isError ? "bg-destructive/10 text-destructive" : "bg-orange-500/10 text-orange-500"
          )}>
            <Terminal className="size-3" />
          </div>
          <span className="text-xs font-semibold">
            {tool.name}
          </span>
        </div>
        
        <div className="flex items-center gap-2">
          <div className={cn(
            "text-[10px] font-bold uppercase tracking-wider px-1.5 py-0.5 rounded-full flex items-center gap-1",
            isSuccess ? "text-primary bg-primary/10" : isError ? "text-destructive bg-destructive/10" : "text-orange-500 bg-orange-500/10"
          )}>
            {isPending && <Loader2 className="size-2.5 animate-spin" />}
            {isSuccess && <CheckCircle2 className="size-2.5" />}
            {isError && <XCircle className="size-2.5" />}
            {tool.status}
          </div>
          {expanded ? <ChevronUp className="size-3 opacity-50" /> : <ChevronDown className="size-3 opacity-50" />}
        </div>
      </div>

      {expanded && (
        <div className="px-3 pb-3 pt-1 space-y-3 animate-in slide-in-from-top-1 duration-200">
          <div className="space-y-1">
            <div className="text-[10px] font-bold text-muted-foreground uppercase tracking-tight">Arguments</div>
            <pre className="text-[11px] font-mono bg-muted p-2 rounded border overflow-x-auto">
              {JSON.stringify(tool.arguments, null, 2)}
            </pre>
          </div>
          
          {tool.result && (
            <div className="space-y-1">
              <div className="text-[10px] font-bold text-muted-foreground uppercase tracking-tight">Result</div>
              <pre className="text-[11px] font-mono bg-muted p-2 rounded border overflow-x-auto whitespace-pre-wrap">
                {tool.result}
              </pre>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
