/**
 * MessageList - Clean message list with auto-scroll
 * 
 * Simplified: no more mode props, just messages
 */

import React, { useRef, useEffect } from "react";
import { useStore } from "../../state/store";
import MessageBubble from "./MessageBubble";
import { Bot, Sparkles } from "lucide-react";

export default function MessageList() {
  const messages = useStore((s) => s.messages);
  const bottomRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom on new messages
  useEffect(() => {
    if (bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [messages.length, messages[messages.length - 1]?.content]);

  // Empty state
  if (messages.length === 0) {
    return (
      <div className="h-full flex flex-col items-center justify-center p-8 text-center">
        <div className="size-16 rounded-2xl bg-gradient-to-br from-emerald-500/10 to-teal-500/10 border border-emerald-500/10 flex items-center justify-center mb-4">
          <Sparkles className="size-8 text-emerald-500/60" />
        </div>
        <h3 className="text-lg font-semibold text-foreground mb-2">
          What would you like to build?
        </h3>
        <p className="text-sm text-muted-foreground max-w-xs">
          Describe your project, ask questions, or run commands like{" "}
          <code className="text-xs bg-muted/50 px-1.5 py-0.5 rounded">/init</code>
        </p>
      </div>
    );
  }

  return (
    <div className="h-full w-full overflow-y-auto overflow-x-hidden">
      <div className="flex flex-col py-4 px-4">
        {messages.map((msg, index) => (
          <MessageBubble
            key={msg.id}
            message={msg}
            index={index}
          />
        ))}
        <div ref={bottomRef} className="h-4 flex-shrink-0" />
      </div>
    </div>
  );
}