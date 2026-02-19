/**
 * TimelineEvent - Shows real-time progress of operations
 * 
 * Displays: received → plan → tool → stream → approval → result → error
 */

import React from "react";
import { cn } from "@/lib/utils";
import {
  CheckCircle2,
  Circle,
  Loader2,
  AlertCircle,
  PlayCircle,
  FileText,
  Brain,
  ChevronDown,
  ChevronUp,
} from "lucide-react";

export interface TimelineEvent {
  id: string;
  stage: "received" | "plan" | "tool" | "stream" | "approval" | "result" | "error" | "cancel";
  title: string;
  detail?: string;
  turnId?: string;
  timestamp: number;
}

interface TimelineEventProps {
  event: TimelineEvent;
  compact?: boolean;
}

const STAGE_CONFIG: Record<TimelineEvent["stage"], {
  icon: React.ComponentType<{ className?: string }>;
  color: string;
  label: string;
}> = {
  received: { icon: PlayCircle, color: "text-blue-500", label: "Received" },
  plan: { icon: Brain, color: "text-purple-500", label: "Planning" },
  tool: { icon: FileText, color: "text-orange-500", label: "Tool Call" },
  stream: { icon: Loader2, color: "text-cyan-500 animate-spin", label: "Streaming" },
  approval: { icon: Circle, color: "text-yellow-500", label: "Awaiting Approval" },
  result: { icon: CheckCircle2, color: "text-green-500", label: "Completed" },
  error: { icon: AlertCircle, color: "text-red-500", label: "Error" },
  cancel: { icon: Circle, color: "text-muted-foreground", label: "Cancelled" },
};

export function TimelineEventItem({ event, compact }: TimelineEventProps) {
  const [expanded, setExpanded] = React.useState(false);
  const config = STAGE_CONFIG[event.stage];
  const Icon = config.icon;
  
  const time = new Date(event.timestamp).toLocaleTimeString("en-US", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });

  return (
    <div className={cn(
      "flex gap-2 py-1.5 px-2 rounded-lg transition-colors",
      "hover:bg-muted/30"
    )}>
      {/* Icon */}
      <div className={cn("shrink-0 mt-0.5", config.color)}>
        <Icon className="size-4" />
      </div>

      {/* Content */}
      <div className="flex-1 min-w-0">
        <div className="flex items-center justify-between gap-2">
          <span className="text-xs font-medium truncate">{event.title}</span>
          <span className="text-[10px] text-muted-foreground shrink-0">{time}</span>
        </div>

        {/* Detail - expandable */}
        {event.detail && (
          <div className="mt-0.5">
            {compact ? (
              <button
                onClick={() => setExpanded(!expanded)}
                className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
              >
                {expanded ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
                {expanded ? "Hide" : "Show"} details
              </button>
            ) : null}
            
            {(expanded || !compact) && (
              <p className="text-[10px] text-muted-foreground mt-0.5 line-clamp-2">
                {event.detail}
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

interface TimelineProps {
  events: TimelineEvent[];
  maxVisible?: number;
}

export function Timeline({ events, maxVisible = 5 }: TimelineProps) {
  const [showAll, setShowAll] = React.useState(false);
  
  if (events.length === 0) return null;
  
  const visibleEvents = showAll ? events : events.slice(-maxVisible);
  const hasMore = events.length > maxVisible && !showAll;
  
  return (
    <div className="border border-border/50 rounded-lg bg-card/30 overflow-hidden">
      <div className="px-2 py-1.5 border-b border-border/50 bg-muted/20">
        <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
          Activity Timeline
        </span>
      </div>
      
      <div className="max-h-[200px] overflow-y-auto">
        {visibleEvents.map((event) => (
          <TimelineEventItem key={event.id} event={event} compact />
        ))}
      </div>
      
      {hasMore && (
        <button
          onClick={() => setShowAll(true)}
          className="w-full py-1.5 text-[10px] text-muted-foreground hover:text-foreground hover:bg-muted/20 transition-colors"
        >
          Show {events.length - maxVisible} more events
        </button>
      )}
    </div>
  );
}

export default Timeline;