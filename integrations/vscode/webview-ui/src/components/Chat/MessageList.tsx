import React, { useRef, useEffect } from "react";
import { useStore } from "../../state/store";
import MessageBubble from "./MessageBubble";

export default function MessageList() {
  const messages = useStore((s) => s.messages);
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  if (messages.length === 0) {
    return (
      <div className="message-list">
        <div className="empty-state" style={{ margin: "auto" }}>
          Ask Sentinel to validate actions, check alignment, or plan your next
          steps.
        </div>
      </div>
    );
  }

  return (
    <div className="message-list">
      {messages.map((msg, index) => (
        <MessageBubble key={msg.id} message={msg} index={index} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
