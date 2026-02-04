import React from "react";
import type { ChatMessage } from "../../state/types";
import { renderMarkdown } from "../../utils/markdown";
import ToolCallCard from "./ToolCallCard";
import FileApproval from "../Actions/FileApproval";

interface Props {
  message: ChatMessage;
  index: number;
}

export default function MessageBubble({ message, index }: Props) {
  const isUser = message.role === "user";
  const delay = `${Math.min(index * 40, 240)}ms`;

  return (
    <div className={`message-row ${isUser ? "message-row--user" : ""}`}>
      <div
        className={`message-bubble message-bubble--enter ${isUser ? "message-bubble--user" : ""}`}
        style={{ animationDelay: delay }}
      >
        {!isUser && <div className="message-meta">Sentinel</div>}

        <div
          dangerouslySetInnerHTML={{ __html: renderMarkdown(message.content) }}
          style={{ overflow: "hidden" }}
        />

        {message.toolCalls?.map((tool, i) => (
          <ToolCallCard key={i} tool={tool} />
        ))}

        {message.fileOperations?.map((op, i) => (
          <FileApproval key={i} operation={op} messageId={message.id} />
        ))}

        {message.streaming && <span className="message-stream" />}
      </div>
    </div>
  );
}
