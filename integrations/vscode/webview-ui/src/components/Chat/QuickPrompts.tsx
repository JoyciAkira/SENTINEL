import React from "react";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";
import { Sparkles, Terminal, Shield, Target, Brain, HelpCircle, PlayCircle } from "lucide-react";
import { cn } from "@/lib/utils";

const PROMPTS = [
  { label: "Initialize", text: "/init", icon: Sparkles },
  { label: "Alignment Status", text: "Show current alignment status", icon: Shield },
  { label: "List Goals", text: "What are the active goals?", icon: Target },
  { label: "Verify Action", text: "Validate my next changes", icon: Terminal },
  { label: "Execute First Pending", text: "/execute-first-pending", icon: PlayCircle },
  { label: "Memory Status", text: "/memory-status", icon: Brain },
  { label: "Help", text: "/help", icon: HelpCircle },
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
    <div className="flex flex-wrap gap-2 mb-4 animate-in fade-in slide-in-from-bottom-1 duration-500 delay-100 fill-mode-both">
      {PROMPTS.map((prompt) => (
        <button
          key={prompt.label}
          type="button"
          disabled={!connected}
          onClick={() => handleSend(prompt.text)}
          className={cn(
            "flex items-center gap-1.5 px-3 py-1.5 rounded-full border bg-card/40 text-[11px] font-medium transition-all shadow-sm",
            "hover:bg-accent hover:border-primary/20 hover:text-primary hover:-translate-y-0.5",
            "disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:translate-y-0"
          )}
        >
          <prompt.icon className="size-3" />
          {prompt.label}
        </button>
      ))}
    </div>
  );
}
