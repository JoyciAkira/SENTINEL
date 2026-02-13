import { useEffect, useState } from "react";
import { useVSCodeAPI } from "./useVSCodeAPI";
import { useMCPMessages } from "./useMCPMessages";

export interface PinnedTranscript {
  transcript_id: string;
  run_id: string;
  frames: Frame[];
  anchors: Anchor[];
  metadata: TranscriptMetadata;
}

export interface Frame {
  frame_id: string;
  frame_type: "summary" | "milestone" | "error" | "decision";
  timestamp: string;
  content: FrameContent;
  turn_range: [number, number];
  token_estimate: number;
}

export interface FrameContent {
  summary: string;
  key_points: string[];
  outcome?: string;
}

export interface AnchorRef {
  anchor_id: string;
  turn_number: number;
  relevance_score: number;
  jump_uri: string;
}

export interface TranscriptMetadata {
  total_turns: number;
  compressed_ratio: number;
  last_updated: string;
  compression_level: "L0" | "L1" | "L2";
}

/**
 * Hook for accessing pinned lightweight transcript
 */
export function usePinnedTranscript() {
  const [transcript, setTranscript] = useState<PinnedTranscript | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const vscodeApi = useVSCodeAPI();
  const { sendMessage, onMessage } = useMCPMessages();

  useEffect(() => {
    // Request pinned transcript on mount
    const requestTranscript = () => {
      sendMessage({
        type: "get_pinned_transcript",
      });
    };

    const handleResponse = (message: any) => {
      if (message.type === "pinned_transcript") {
        setTranscript(message.data);
        setLoading(false);
      } else if (message.type === "error") {
        setError(message.data || "Failed to load transcript");
        setLoading(false);
      }
    };

    onMessage("pinned_transcript", handleResponse);

    // Also listen for updates
    onMessage("pinned_transcript_update", (message: any) => {
      if (message.data) {
        setTranscript(message.data);
      }
    });

    requestTranscript();

    return () => {
      // Cleanup listeners
      // (implementation would remove listeners)
    };
  }, [vscodeApi, onMessage]);

  return {
    transcript,
    loading,
    error,
    refresh: () => requestTranscript(),
  };
}
