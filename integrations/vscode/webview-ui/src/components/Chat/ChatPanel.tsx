import React, { useEffect, useState } from "react";
import MessageList from "./MessageList";
import ChatInput from "./ChatInput";
import QuickPrompts from "./QuickPrompts";
import { useStore } from "../../state/store";
import GoalBuilder from "../Goals/GoalBuilder";

interface Props {
  previewVisible: boolean;
  onTogglePreview: () => void;
}

export default function ChatPanel({ previewVisible, onTogglePreview }: Props) {
  const alignment = useStore((s) => s.alignment);
  const goals = useStore((s) => s.goals);
  const [showGoalBuilder, setShowGoalBuilder] = useState(goals.length === 0);

  useEffect(() => {
    if (goals.length === 0) {
      setShowGoalBuilder(true);
    }
  }, [goals.length]);

  return (
    <div className="chat-shell">
      <div className="chat-header">
        <div>
          <div className="chat-title">Sentinel Chat</div>
          <div className="chat-subtitle">
            Supervised reasoning and gated actions
          </div>
        </div>
        <div className="chat-header__actions">
          <span className="chip chip--muted">
            Alignment {alignment ? `${alignment.score.toFixed(1)}%` : "--"}
          </span>
          <button
            type="button"
            className="toggle toggle--mini"
            data-active={previewVisible}
            onClick={onTogglePreview}
          >
            Preview {previewVisible ? "On" : "Off"}
          </button>
          <div className="status-pill">Policy: Guarded</div>
        </div>
      </div>
      <div className="chat-scroll">
        <div className="goal-guide">
          <div className="goal-guide__header">
            <div>
              <div className="section-title">Inline Goal Builder</div>
              <div className="chat-subtitle">
                Define or refine goals before running any actions.
              </div>
            </div>
            <button
              type="button"
              className="btn btn--ghost"
              onClick={() => setShowGoalBuilder((open) => !open)}
            >
              {showGoalBuilder ? "Hide Builder" : "Edit Goals"}
            </button>
          </div>
          {showGoalBuilder && <GoalBuilder />}
        </div>
        <QuickPrompts />
        <MessageList />
      </div>
      <ChatInput />
    </div>
  );
}
