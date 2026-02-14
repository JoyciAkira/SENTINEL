import { useCallback } from 'react';
import { useStore } from '../state/store';
import { useVSCodeAPI } from './useVSCodeAPI';

/**
 * Hook for accessing quality dashboard from Zustand store.
 * Sends a refresh request to extension host via postMessage
 * and reads latest report from centralized store.
 *
 * UPDATED: Now supports retrieving specific reports by ID via the quality_report tool
 */
export function useQualityReport() {
  const qualityDashboard = useStore((s) => s.qualityDashboard);
  const qualityStatus = useStore((s) => s.qualityStatus);
  const vscodeApi = useVSCodeAPI();

  const refresh = useCallback(() => {
    vscodeApi.postMessage({ type: 'requestQualityReport' });
  }, [vscodeApi]);

  const report = qualityDashboard?.latest_report ?? null;
  const loading = !qualityDashboard && !qualityStatus;

  const overallScore = report
    ? report.scores.reduce((sum, s) => {
        const weights: Record<string, number> = {
          Correctness: 0.30,
          Reliability: 0.25,
          Maintainability: 0.20,
          Security: 0.15,
          UXDevEx: 0.10,
        };
        return sum + s.value * (weights[s.metric] ?? 0.2);
      }, 0)
    : 0;

  const passRate = report
    ? (report.scores.filter((s) => s.result === 'Pass').length / report.scores.length) * 100
    : 0;

  return { report, loading, error: null, overallScore, passRate, refresh, qualityStatus };
}
