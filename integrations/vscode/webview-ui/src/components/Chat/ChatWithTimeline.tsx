/**
 * ChatWithTimeline - Combines messages with real-time timeline
 * 
 * Shows:
 * - Activity timeline (what's happening now)
 * - Messages (conversation history)
 * - Streaming indicator
 */

import React from "react";
import { useStore } from "../../state/store";
import MessageList from "./MessageList";
import QuickPrompts from "./QuickPrompts";
import { Timeline, type TimelineEvent } from "./TimelineEvent";
import { cn } from "@/lib/utils";
import { Loader2, Activity } from "lucide-react";

interface ChatWithTimelineProps {
  showQuickPrompts?: boolean;
  goalsCount?: number;
  pendingApprovals?: number;
  alignmentScore?: number;
}

export function ChatWithTimeline({
  showQuickPrompts = true,
  goalsCount,
  pendingApprovals,
  alignmentScore,
}: ChatWithTimelineProps) {
  const messages = useStore((s) => s.messages);
  const timeline = useStore((s) => s.timeline);
  
  // Check for active streaming
  const isStreaming = messages.some((m) => m.streaming);
  
  // Convert timeline state to component format
  const timelineEvents: TimelineEvent[] = timeline.map((e) => ({
    id: e.id,
    stage: e.stage,
    title: e.title,
    detail: e.detail,
    turnId: e.turnId,
    timestamp: e.timestamp,
  }));

  // Get the last few events for mini view
  const recentEvents = timelineEvents.slice(-3);
  const hasActivity = recentEvents.length > 0;

  return (
    <div className="flex flex-col h-full">
      {/* Mini Activity Bar - shows when there's recent activity */}
      {hasActivity && (
        <div className="shrink-0 border-b border-border/30 bg-muted/10 px-3 py-2">
          <div className="flex items-center gap-2">
            <Activity className="size-3 text-muted-foreground" />
            <div className="flex-1 flex items-center gap-2 overflow-x-auto">
              {recentEvents.map((event) => (
                <div
                  key={event.id}
                  className={cn(
                    "flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px]",
                    event.stage === "error" && "bg-red-500/10 text-red-500",
                    event.stage === "result" && "bg-green-500/10 text-green-600",
                    event.stage === "stream" && "bg-cyan-500/10 text-cyan-600",
                    (event.stage === "plan" || event.stage === "tool") && "bg-orange-500/10 text-orange-600",
                    (event.stage === "received" || event.stage === "approval") && "bg-muted/30 text-muted-foreground"
                  )}
                >
                  {event.stage === "stream" && <Loader2 className="size-2.5 animate-spin" />}
                  <span className="truncate max-w-[120px]">{event.title}</span>
                </div>
              ))}
            </div>
          </div>
        </div>
      )}

      {/* Messages Area */}
      <div className="flex-1 min-h-0">
        {messages.length === 0 && showQuickPrompts ? (
          <div className="h-full flex flex-col">
            <div className="flex-1" />
            <QuickPrompts
              goalsCount={goalsCount}
              pendingApprovals={pendingApprovals}
              alignmentScore={alignmentScore}
              hasConversation={false}
            />
          </div>
        ) : (
          <MessageList />
        )}
      </div>

      {/* Full Timeline - collapsible, shown when there are events */}
      {timelineEvents.length > 3 && (
        <div className="shrink-0 border-t border-border/30 px-3 py-2 max-h-[150px] overflow-y-auto">
          <Timeline events={timelineEvents} maxVisible={10} />
        </div>
      )}
    </div>
  );
}

export default ChatWithTimeline;