import React, { useRef, useCallback } from "react";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";

export default function ChatInput() {
  const inputText = useStore((s) => s.inputText);
  const setInputText = useStore((s) => s.setInputText);
  const addMessage = useStore((s) => s.addMessage);
  const connected = useStore((s) => s.connected);
  const vscode = useVSCodeAPI();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const send = useCallback(() => {
    const text = inputText.trim();
    if (!text || !connected) return;

    addMessage({
      id: crypto.randomUUID(),
      role: "user",
      content: text,
      timestamp: Date.now(),
    });

    vscode.postMessage({
      type: "chatMessage",
      text,
    });

    setInputText("");

    // Reset textarea height
    if (textareaRef.current) {
      textareaRef.current.style.height = "auto";
    }
  }, [inputText, connected, addMessage, setInputText, vscode]);

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      send();
    }
  };

  const handleInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    setInputText(e.target.value);
    // Auto-grow up to 5 lines
    const el = e.target;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 100) + "px";
  };

  return (
    <div className="chat-input">
      <div className="chat-input__row">
        <textarea
          ref={textareaRef}
          value={inputText}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          placeholder={connected ? "Ask Sentinel..." : "Not connected"}
          disabled={!connected}
          rows={1}
          className="chat-textarea"
        />
        <button
          onClick={send}
          disabled={!connected || !inputText.trim()}
          className="btn"
        >
          Send
        </button>
      </div>
    </div>
  );
}
