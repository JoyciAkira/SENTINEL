/**
 * MessageBubble - Clean, Cline-like message component
 * 
 * Design principles:
 * - Conversational first: content is king
 * - Progressive disclosure: details on demand
 * - No artificial "outcome boxes"
 */

import React, { useState, useMemo } from "react";
import type { ChatMessage } from "../../state/types";
import { renderMarkdown } from "../../utils/markdown";
import FileApproval from "../Actions/FileApproval";
import { ReasoningTrace, type ReasoningTraceData } from "../ReasoningTrace";
import ChoiceButtons, { hasChoices, extractQuestionBeforeChoices } from "./ChoiceButtons";
import { cn } from "@/lib/utils";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";
import { Button } from "../ui/button";
import { ChevronDown, ChevronUp, Copy, Check, Brain, FileText } from "lucide-react";

interface MessageBubbleProps {
  message: ChatMessage;
  index: number;
}

export default function MessageBubble({ message, index }: MessageBubbleProps) {
  const isUser = message.role === "user";
  const vscode = useVSCodeAPI();
  
  // State
  const [copied, setCopied] = useState(false);
  const [showReasoning, setShowReasoning] = useState(false);
  const [showFileOps, setShowFileOps] = useState(true);
  
  // Derived
  const hasFileOperations = (message.fileOperations?.length ?? 0) > 0;
  const pendingOperations = message.fileOperations?.filter(op => op.approved === undefined) ?? [];
  const hasReasoning = (message.thoughtChain?.length ?? 0) > 0;
  const containsChoices = !isUser && hasChoices(message.content);
  const displayContent = containsChoices ? extractQuestionBeforeChoices(message.content) : message.content;
  
  // Build reasoning trace data
  const reasoningTrace: ReasoningTraceData | null = useMemo(() => {
    if (!message.thoughtChain || message.thoughtChain.length === 0) return null;
    
    return {
      query: message.content.slice(0, 100),
      steps: message.thoughtChain.map((thought, i) => ({
        action: `Step ${i + 1}`,
        observation: thought,
        decision: "Proceeding with analysis",
      })),
      confidence: 0.85,
      rationale: message.content.slice(0, 200),
    };
  }, [message.thoughtChain, message.content]);

  // Handlers
  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(message.content);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // Fallback for webviews without clipboard API
      const textarea = document.createElement("textarea");
      textarea.value = message.content;
      textarea.style.cssText = "position:fixed;opacity:0";
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      document.body.removeChild(textarea);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
  };

  return (
    <div 
      className={cn(
        "flex w-full gap-3 py-3 animate-in fade-in slide-in-from-bottom-2 duration-300",
        isUser ? "justify-end" : "justify-start"
      )}
      style={{ animationDelay: `${Math.min(index * 40, 200)}ms` }}
    >
      {/* Avatar */}
      <div className={cn("shrink-0", isUser && "order-2")}>
        <div className={cn(
          "size-8 rounded-full flex items-center justify-center text-xs font-semibold",
          "border shadow-sm",
          isUser 
            ? "bg-primary/10 border-primary/20 text-primary" 
            : "bg-gradient-to-br from-emerald-500/20 to-teal-500/20 border-emerald-500/20 text-emerald-600 dark:text-emerald-400"
        )}>
          {isUser ? "U" : "S"}
        </div>
      </div>

      {/* Message Content */}
      <div className={cn("flex-1 max-w-[85%] space-y-2", isUser && "order-1")}>
        {/* Header - AI only */}
        {!isUser && (
          <div className="flex items-center justify-between gap-2 px-1">
            <span className="text-xs font-medium text-muted-foreground">Sentinel</span>
            <div className="flex items-center gap-1">
              <Button
                size="icon"
                variant="ghost"
                onClick={handleCopy}
                className="size-6 opacity-50 hover:opacity-100"
              >
                {copied ? <Check className="size-3" /> : <Copy className="size-3" />}
              </Button>
            </div>
          </div>
        )}

        {/* Main Content Bubble */}
        <div className={cn(
          "rounded-2xl px-4 py-3 shadow-sm border",
          isUser 
            ? "bg-primary/5 border-primary/10 rounded-tr-sm" 
            : "bg-card border-border rounded-tl-sm"
        )}>
          {/* Markdown Content */}
          <div 
            className={cn(
              "prose prose-sm dark:prose-invert max-w-none",
              "prose-p:my-0 prose-p:leading-relaxed",
              "prose-headings:mt-4 prose-headings:mb-2 first:prose-headings:mt-0",
              "prose-code:before:hidden prose-code:after:hidden",
              "prose-pre:my-2 prose-pre:p-3 prose-pre:rounded-lg prose-pre:bg-muted/50",
              "sentinel-selectable-content"
            )}
            dangerouslySetInnerHTML={{ __html: renderMarkdown(displayContent) }}
          />

          {/* Streaming cursor */}
          {message.streaming && (
            <span className="inline-block w-1.5 h-4 ml-1 bg-primary/50 animate-pulse align-middle" />
          )}
          
          {/* Choice Buttons - Interactive A/B/C options */}
          {containsChoices && !message.streaming && (
            <ChoiceButtons 
              content={message.content} 
              messageId={message.id} 
            />
          )}
        </div>

        {/* Reasoning Trace - Collapsible */}
        {!isUser && hasReasoning && (
          <div className="ml-1">
            <button
              onClick={() => setShowReasoning(!showReasoning)}
              className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors py-1"
            >
              <Brain className="size-3" />
              <span>{showReasoning ? "Hide reasoning" : "Show reasoning"}</span>
              {showReasoning ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
            </button>
            
            {showReasoning && reasoningTrace && (
              <div className="mt-2 animate-in fade-in slide-in-from-top-2 duration-200">
                <ReasoningTrace trace={reasoningTrace} />
              </div>
            )}
          </div>
        )}

        {/* File Operations - Inline */}
        {!isUser && hasFileOperations && (
          <div className="ml-1 space-y-2">
            <button
              onClick={() => setShowFileOps(!showFileOps)}
              className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors py-1"
            >
              <FileText className="size-3" />
              <span>
                {pendingOperations.length > 0 
                  ? `${pendingOperations.length} file${pendingOperations.length > 1 ? "s" : ""} pending approval`
                  : `${message.fileOperations!.length} file${message.fileOperations!.length > 1 ? "s" : ""} changed`
              }
              </span>
              {showFileOps ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
            </button>

            {showFileOps && (
              <div className="space-y-2 animate-in fade-in slide-in-from-top-2 duration-200">
                {message.fileOperations!.map((op, i) => (
                  <FileApproval 
                    key={`${op.path}-${i}`} 
                    operation={op} 
                    messageId={message.id} 
                  />
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}