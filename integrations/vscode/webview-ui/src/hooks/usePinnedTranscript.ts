import { useCallback } from "react";
import { useStore } from "../state/store";
import { useVSCodeAPI } from "./useVSCodeAPI";

/**
 * Hook for accessing the pinned transcript from Zustand store.
 * Sends a refresh request to the extension host via postMessage
 * and reads the transcript state from the centralized store.
 */
export function usePinnedTranscript() {
  const pinnedTranscript = useStore((s) => s.pinnedTranscript);
  const vscodeApi = useVSCodeAPI();

  const refresh = useCallback(() => {
    vscodeApi.postMessage({ type: "requestPinnedTranscript" });
  }, [vscodeApi]);

  return {
    transcript: pinnedTranscript,
    loading: !pinnedTranscript,
    error: null,
    refresh,
  };
}
