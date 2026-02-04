import React from "react";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";

const PROMPTS = [
  { label: "Init", text: "/init" },
  { label: "Status", text: "Show alignment status" },
  { label: "Goals", text: "List current goals" },
  { label: "Validate", text: "Validate next action" },
];

export default function QuickPrompts() {
  const connected = useStore((s) => s.connected);
  const addMessage = useStore((s) => s.addMessage);
  const vscode = useVSCodeAPI();

  const handleSend = (text: string) => {
    if (!connected) return;

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
  };

  return (
    <div className="quick-prompts">
      {PROMPTS.map((prompt) => (
        <button
          key={prompt.label}
          type="button"
          className="quick-prompt"
          disabled={!connected}
          onClick={() => handleSend(prompt.text)}
        >
          {prompt.label}
        </button>
      ))}
    </div>
  );
}
