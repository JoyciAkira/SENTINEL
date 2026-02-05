import React, { useState } from "react";
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
  const [thoughtsExpanded, setThoughtsExpanded] = useState(false);

  const hasThoughts = message.thoughtChain && message.thoughtChain.length > 0;

  return (
    <div className={`message-row ${isUser ? "message-row--user" : ""}`}>
      <div
        className={`message-bubble message-bubble--enter ${isUser ? "message-bubble--user" : ""}`}
        style={{ animationDelay: delay }}
      >
        {!isUser && <div className="message-meta">Sentinel</div>}

        {/* Chain of Thought Visualization */}
        {!isUser && hasThoughts && (
          <div style={{ marginBottom: '12px', fontSize: '12px' }}>
            <div 
              onClick={() => setThoughtsExpanded(!thoughtsExpanded)}
              style={{ 
                cursor: 'pointer', 
                color: 'var(--text-subtle)', 
                display: 'flex', 
                alignItems: 'center', 
                gap: '6px' 
              }}
            >
              <span style={{ fontSize: '10px' }}>{thoughtsExpanded ? '▼' : '▶'}</span>
              <span className="mono">Running cognitive check...</span>
            </div>
            
            {thoughtsExpanded && (
              <div style={{
                marginTop: '8px',
                padding: '8px 12px',
                borderLeft: '2px solid var(--accent)',
                background: 'rgba(15, 118, 110, 0.05)',
                borderRadius: '0 6px 6px 0',
                fontFamily: 'var(--font-mono)',
                fontSize: '11px',
                color: 'var(--text-muted)'
              }}>
                {message.thoughtChain!.map((thought, i) => (
                  <div key={i} style={{ marginBottom: '4px' }}>• {thought}</div>
                ))}
              </div>
            )}
          </div>
        )}

        <div
          dangerouslySetInnerHTML={{ __html: renderMarkdown(message.content) }}
          style={{ overflow: "hidden", lineHeight: 1.5 }}
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
