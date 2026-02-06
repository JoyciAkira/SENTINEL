import React, { useRef, useCallback } from "react";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";
import { Send, Command, CornerDownLeft } from "lucide-react";
import { Button } from "../ui/button";
import { cn } from "@/lib/utils";

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

    if (text === "/clear-memory") {
      vscode.postMessage({ type: "clearChatMemory" });
      setInputText("");
      if (textareaRef.current) {
        textareaRef.current.style.height = "auto";
      }
      return;
    }

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
    const el = e.target;
    el.style.height = "auto";
    el.style.height = Math.min(el.scrollHeight, 150) + "px";
  };

  return (
    <div className="relative group animate-in slide-in-from-bottom-2 duration-500 delay-150 fill-mode-both">
      <div className={cn(
        "relative flex flex-col rounded-xl border bg-card/50 shadow-sm transition-all overflow-hidden",
        "focus-within:border-primary/30 focus-within:ring-2 focus-within:ring-primary/10",
        !connected && "opacity-50 cursor-not-allowed"
      )}>
        <textarea
          ref={textareaRef}
          value={inputText}
          onChange={handleInput}
          onKeyDown={handleKeyDown}
          placeholder={connected ? "Ask Sentinel anything..." : "Establishing connection..."}
          disabled={!connected}
          rows={1}
          className="w-full bg-transparent border-none focus:ring-0 text-sm px-4 pt-4 pb-12 resize-none min-h-[56px] placeholder:text-muted-foreground outline-none"
        />
        
        <div className="absolute left-3 bottom-3 flex items-center gap-2">
           <div className="flex items-center gap-1.5 px-2 py-1 rounded bg-accent/30 border border-border text-[10px] text-muted-foreground font-medium">
              <Command className="size-2.5" />
              <span>Context Active</span>
           </div>
        </div>

        <div className="absolute right-3 bottom-3 flex items-center gap-3">
          <div className="hidden sm:flex items-center gap-1 text-[10px] text-muted-foreground font-medium mr-1 opacity-50">
            <CornerDownLeft className="size-2.5" />
            <span>Send</span>
          </div>
          <Button 
            size="icon-xs" 
            onClick={send}
            disabled={!connected || !inputText.trim()}
            className={cn(
              "rounded-lg transition-all",
              inputText.trim() ? "bg-primary scale-100" : "bg-muted scale-95"
            )}
          >
            <Send className="size-3" />
          </Button>
        </div>
      </div>
      
      {/* Decorative gradient border */}
      <div className="absolute -inset-px rounded-xl border-primary/20 pointer-events-none opacity-0 group-focus-within:opacity-100 transition-opacity" />
    </div>
  );
}
