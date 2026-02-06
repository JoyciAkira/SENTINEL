import React, { useState } from "react";
import type { ChatMessage } from "../../state/types";
import { renderMarkdown } from "../../utils/markdown";
import ToolCallCard from "./ToolCallCard";
import FileApproval from "../Actions/FileApproval";
import { cn } from "@/lib/utils";

export default function MessageBubble({ message, index }: { message: ChatMessage; index: number }) {
  const isUser = message.role === "user";
  const [thoughtsExpanded, setThoughtsExpanded] = useState(false);
  const hasThoughts = message.thoughtChain && message.thoughtChain.length > 0;
  const hasExplainability = Boolean(message.explainability);

  return (
    <div className={cn(
      "flex w-full animate-in fade-in slide-in-from-bottom-3 duration-500 fill-mode-both",
      isUser ? "justify-end" : "justify-start"
    )} style={{ animationDelay: `${Math.min(index * 50, 300)}ms` }}>
      <div className={cn(
        "flex max-w-[85%] gap-3",
        isUser ? "flex-row-reverse" : "flex-row"
      )}>
        {/* AVATAR */}
        <div className="shrink-0 mt-1">
          <div className={cn(
            "size-7 rounded-lg flex items-center justify-center text-[10px] font-bold shadow-sm border",
            isUser ? "bg-primary text-primary-foreground border-primary/20" : "bg-card text-foreground border-border"
          )}>
            {isUser ? 'U' : 'S'}
          </div>
        </div>

        {/* CONTENT */}
        <div className={cn("space-y-2 w-full", isUser ? "items-end" : "items-start")}>
          <div className={cn(
            "px-4 py-3 rounded-2xl shadow-sm text-sm leading-relaxed border",
            isUser 
              ? "bg-primary/5 border-primary/10 text-foreground rounded-tr-none" 
              : "bg-card border-border text-foreground rounded-tl-none"
          )}>
            {!isUser && (
              <div className="text-[10px] font-bold uppercase tracking-widest text-primary mb-1.5 opacity-80 flex justify-between items-center">
                <span>Sentinel AI</span>
                {hasThoughts && (
                  <button 
                    onClick={() => setThoughtsExpanded(!thoughtsExpanded)}
                    className="text-[9px] hover:underline normal-case font-normal opacity-60"
                  >
                    {thoughtsExpanded ? "Hide thoughts" : "Show thoughts"}
                  </button>
                )}
                {hasExplainability && (
                  <span className="text-[9px] normal-case opacity-60">Explainable turn</span>
                )}
              </div>
            )}

            {/* Thought Chain */}
            {!isUser && hasThoughts && thoughtsExpanded && (
              <div className="mb-3 p-2 bg-muted/50 rounded-lg border border-border/50 font-mono text-[11px] text-muted-foreground animate-in fade-in zoom-in-95 duration-200">
                {message.thoughtChain!.map((thought, i) => (
                  <div key={i} className="mb-1 last:mb-0">• {thought}</div>
                ))}
              </div>
            )}

            <div 
              className="prose prose-sm dark:prose-invert max-w-none prose-p:leading-relaxed prose-pre:bg-muted prose-pre:border prose-pre:rounded-lg"
              dangerouslySetInnerHTML={{ __html: renderMarkdown(message.content) }} 
            />
            
            {message.streaming && (
              <span className="inline-block w-1.5 h-4 ml-1 bg-primary/40 animate-pulse align-middle" />
            )}
          </div>

          {!isUser && message.explainability && (
            <div className="text-[11px] border rounded-lg p-2 bg-card/60 space-y-1">
              <div className="font-semibold text-[10px] uppercase tracking-wider text-muted-foreground">Turn Explainability</div>
              {message.explainability.intent_summary && (
                <div><span className="text-muted-foreground">Intent:</span> {message.explainability.intent_summary}</div>
              )}
              {typeof message.explainability.alignment_score === "number" && (
                <div>
                  <span className="text-muted-foreground">Alignment:</span>{" "}
                  {message.explainability.alignment_score.toFixed(1)}%
                </div>
              )}
              {typeof message.explainability.reliability_healthy === "boolean" && (
                <div>
                  <span className="text-muted-foreground">Reliability:</span>{" "}
                  {message.explainability.reliability_healthy ? "Healthy" : "Violated"}
                </div>
              )}
              {message.explainability.evidence?.length ? (
                <div className="text-muted-foreground">
                  Evidence: {message.explainability.evidence.join(" | ")}
                </div>
              ) : null}
            </div>
          )}

          {/* TOOL CALLS & ACTIONS */}
          {(message.toolCalls || message.fileOperations) && (
            <div className="space-y-2 mt-2 w-full">
              {message.toolCalls?.map((tool, i) => (
                <div key={`tool-${i}`} className="animate-in fade-in slide-in-from-left-2 duration-300 fill-mode-both" style={{ animationDelay: '150ms' }}>
                   <ToolCallCard tool={tool} />
                </div>
              ))}
              {message.fileOperations?.map((op, i) => (
                <div key={`op-${i}`} className="animate-in fade-in slide-in-from-left-2 duration-300 fill-mode-both" style={{ animationDelay: '200ms' }}>
                  <FileApproval operation={op} messageId={message.id} />
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
