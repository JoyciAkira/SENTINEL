/**
 * PinnedTranscript Component
 *
 * Displays the Pinned Lightweight Transcript (PLT) showing memory compaction
 * with frames, anchors, and compression metrics.
 */

import React, { useMemo } from 'react';
import { useStore } from '../../state/store';
import {
  FileText,
  Minimize2,
  Zap,
  Clock,
  Hash,
  Target,
  AlertCircle,
  CheckCircle2,
  ChevronRight,
  ChevronDown,
} from 'lucide-react';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '../ui/card';
import { Badge } from '../ui/badge';
import { Button } from '../ui/button';
import { ScrollArea } from '../ui/scroll-area';
import { cn } from '../../lib/utils';

interface PinnedTranscriptProps {
  className?: string;
}

const COMPRESSION_LEVEL_INFO = {
  L0: { label: 'High Quality', color: 'bg-green-500/10 text-green-500', description: 'Minimal compression' },
  L1: { label: 'Standard', color: 'bg-blue-500/10 text-blue-500', description: 'Balanced compression' },
  L2: { label: 'Memory Optimized', color: 'bg-orange-500/10 text-orange-500', description: 'Maximum compression' },
};

const TURN_TYPE_INFO = {
  Normal: { icon: FileText, color: 'text-sentinel-muted-foreground' },
  Decision: { icon: Target, color: 'text-sentinel-accent' },
  Error: { icon: AlertCircle, color: 'text-red-500' },
};

function shortId(id: string): string {
  return id.replace(/-/g, '').slice(0, 8).toUpperCase();
}

export function PinnedTranscript({ className }: PinnedTranscriptProps) {
  const pinnedTranscript = useStore((s) => s.pinnedTranscript);
  const [expandedFrame, setExpandedFrame] = React.useState<string | null>(null);
  const [showAnchorsOnly, setShowAnchorsOnly] = React.useState(false);

  if (!pinnedTranscript) {
    return (
      <Card className={cn('sentinel-pinned-transcript', className)}>
        <CardContent className="flex items-center justify-center py-12">
          <div className="text-center">
            <FileText className="h-12 w-12 mx-auto mb-4 text-sentinel-muted-foreground opacity-50" />
            <p className="text-sentinel-muted-foreground">No transcript data available</p>
            <p className="text-xs text-sentinel-muted-foreground mt-2">
              Transcript data appears after memory compaction
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  const levelInfo = COMPRESSION_LEVEL_INFO[pinnedTranscript.metadata.compression_level];
  const framesToShow = showAnchorsOnly
    ? pinnedTranscript.frames.filter((f) =>
        pinnedTranscript.anchors.some((a) =>
          a.turn_id >= f.turn_range[0] && a.turn_id <= f.turn_range[1]
        )
      )
    : pinnedTranscript.frames;

  return (
    <Card className={cn('sentinel-pinned-transcript', className)}>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <FileText className="h-5 w-5 text-sentinel-accent" />
            <CardTitle>Pinned Transcript</CardTitle>
            <Badge variant="outline" className={levelInfo.color}>
              {levelInfo.label}
            </Badge>
          </div>
          <div className="flex items-center gap-2">
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowAnchorsOnly(!showAnchorsOnly)}
              className="h-8 px-2"
            >
              <Target className="h-4 w-4 mr-1" />
              {showAnchorsOnly ? 'Anchors Only' : 'All Frames'}
            </Button>
          </div>
        </div>
        <CardDescription>
          <div>
            {pinnedTranscript.total_turns} turns compressed into {pinnedTranscript.frames.length} frames
            <div className="flex items-center gap-4 mt-2 text-xs">
              <div className="flex items-center gap-1">
                <Zap className="h-3 w-3 text-sentinel-muted-foreground" />
                <span className="text-sentinel-muted-foreground">
                  {(pinnedTranscript.compression_ratio * 100).toFixed(1)}% compressed
                </span>
              </div>
              <div className="flex items-center gap-1">
                <Clock className="h-3 w-3 text-sentinel-muted-foreground" />
                <span className="text-sentinel-muted-foreground">
                  {new Date(pinnedTranscript.metadata.created_at).toLocaleTimeString()}
                </span>
              </div>
              <div className="flex items-center gap-1">
                <Hash className="h-3 w-3 text-sentinel-muted-foreground" />
                <span className="text-sentinel-muted-foreground font-mono">
                  {shortId(pinnedTranscript.transcript_id)}
                </span>
              </div>
            </div>
          </div>
        </CardDescription>
      </CardHeader>

      <CardContent className="p-0">
        <ScrollArea className="h-[400px]">
          <div className="p-4 space-y-3">
            {/* Anchors Section */}
            {pinnedTranscript.anchors.length > 0 && (
              <div className="mb-4">
                <h4 className="text-xs font-semibold uppercase tracking-wide text-sentinel-muted-foreground mb-2 flex items-center gap-1">
                  <Target className="h-3 w-3" />
                  Key Anchors ({pinnedTranscript.anchors.length})
                </h4>
                <div className="space-y-1">
                  {pinnedTranscript.anchors.map((anchor) => {
                    const TypeInfo = TURN_TYPE_INFO[anchor.turn_type];
                    return (
                      <div
                        key={anchor.turn_id}
                        className="flex items-center gap-2 p-2 rounded-md bg-sentinel-accent/5 border border-sentinel-accent/20"
                      >
                        <TypeInfo.icon className={cn('h-4 w-4', TypeInfo.color)} />
                        <span className="text-sm font-medium">Turn {anchor.turn_id}</span>
                        <Badge variant="outline" className="text-xs">
                          {anchor.turn_type}
                        </Badge>
                        <span className="text-xs text-sentinel-muted-foreground flex-1">
                          {anchor.reason}
                        </span>
                      </div>
                    );
                  })}
                </div>
              </div>
            )}

            {/* Frames Section */}
            <div>
              <h4 className="text-xs font-semibold uppercase tracking-wide text-sentinel-muted-foreground mb-2 flex items-center gap-1">
                <Minimize2 className="h-3 w-3" />
                Frames ({framesToShow.length})
              </h4>
              <div className="space-y-2">
                {framesToShow.map((frame) => {
                  const isExpanded = expandedFrame === frame.frame_id;
                  const hasAnchor = pinnedTranscript.anchors.some(
                    (a) => a.turn_id >= frame.turn_range[0] && a.turn_id <= frame.turn_range[1]
                  );

                  return (
                    <div
                      key={frame.frame_id}
                      className={cn(
                        'border rounded-md transition-colors',
                        hasAnchor
                          ? 'border-sentinel-accent/30 bg-sentinel-accent/5'
                          : 'border-sentinel-muted-foreground/20'
                      )}
                    >
                      <button
                        onClick={() =>
                          setExpandedFrame(isExpanded ? null : frame.frame_id)
                        }
                        className="w-full flex items-center justify-between p-3 text-left hover:bg-sentinel-accent/10 transition-colors"
                      >
                        <div className="flex items-center gap-2 flex-1 min-w-0">
                          {isExpanded ? (
                            <ChevronDown className="h-4 w-4 text-sentinel-muted-foreground flex-shrink-0" />
                          ) : (
                            <ChevronRight className="h-4 w-4 text-sentinel-muted-foreground flex-shrink-0" />
                          )}
                          <div className="flex items-center gap-2 min-w-0">
                            <span className="text-xs font-mono text-sentinel-muted-foreground">
                              {shortId(frame.frame_id)}
                            </span>
                            <span className="text-sm font-medium truncate">
                              Turns {frame.turn_range[0]}-{frame.turn_range[1]}
                            </span>
                          </div>
                        </div>
                        <div className="flex items-center gap-2">
                          {hasAnchor && (
                            <Target className="h-4 w-4 text-sentinel-accent" />
                          )}
                          <Badge variant="outline" className="text-xs">
                            {(frame.compression_ratio * 100).toFixed(0)}%
                          </Badge>
                        </div>
                      </button>

                      {isExpanded && (
                        <div className="px-3 pb-3 pt-1 border-t border-sentinel-muted-foreground/10 mt-2">
                          <p className="text-sm text-sentinel-foreground">
                            {frame.content_summary}
                          </p>
                          <div className="mt-2 text-xs text-sentinel-muted-foreground">
                            Created: {new Date(frame.metadata.created_at).toLocaleString()}
                          </div>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          </div>
        </ScrollArea>
      </CardContent>
    </Card>
  );
}
