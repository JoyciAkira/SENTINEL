import React, { useRef, useEffect } from "react";
import { useStore } from "../../state/store";
import MessageBubble from "./MessageBubble";
import { ScrollArea } from "../ui/scroll-area";
import { Bot, ChevronRight } from "lucide-react";

export default function MessageList({
  compact = false,
  clineMode = false,
}: {
  compact?: boolean;
  clineMode?: boolean;
}) {
  const messages = useStore((s) => s.messages);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  if (messages.length === 0) {
    if (clineMode) {
      return (
        <div className="sentinel-empty-hero">
          <div className="sentinel-empty-hero__icon">
            <Bot className="size-8" />
          </div>
          <h3>What can I do for you?</h3>
          <div className="sentinel-empty-hero__recent">
            <ChevronRight className="size-3" />
            <span>Recent tasks</span>
          </div>
        </div>
      );
    }
    return (
      <div className="h-full flex flex-col items-center justify-center p-8 text-center space-y-4 opacity-60">
        <div className="size-12 rounded-2xl bg-primary/10 flex items-center justify-center">
           <svg className="size-6 text-primary" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M8 10h.01M12 10h.01M16 10h.01M9 16H5a2 2 0 01-2-2V6a2 2 0 012-2h14a2 2 0 012 2v8a2 2 0 01-2 2h-5l-5 5v-5z" />
           </svg>
        </div>
        <div className="space-y-1">
          <p className="text-sm font-semibold text-foreground">No messages yet</p>
          <p className="text-xs text-muted-foreground">Ask Sentinel to validate actions, check alignment, or plan your next steps.</p>
        </div>
      </div>
    );
  }

  return (
    <ScrollArea className="h-full w-full">
      <div className={compact ? "flex flex-col gap-3 p-3" : "flex flex-col gap-6 p-6"}>
        {messages.map((msg, index) => (
          <MessageBubble key={msg.id} message={msg} index={index} compact={compact} />
        ))}
        <div ref={bottomRef} className="h-4" />
      </div>
    </ScrollArea>
  );
}
