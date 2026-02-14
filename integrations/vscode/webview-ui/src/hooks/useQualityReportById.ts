import { useCallback } from 'react';
import { useStore } from '../state/store';
import { useVSCodeAPI } from './useVSCodeAPI';

/**
 * Hook for accessing a specific quality report by ID.
 * Sends a request to extension host via postMessage
 * and reads the specific report from store.
 *
 * This allows retrieving a specific quality report using the quality_report MCP tool
 */
export function useQualityReportById(reportId: string) {
  const qualityDashboard = useStore((s) => s.qualityDashboard);
  const vscodeApi = useVSCodeAPI();

  const requestReport = useCallback(() => {
    vscodeApi.postMessage({
      type: 'requestQualityReport',
      report_id: reportId,
    });
  }, [vscodeApi, reportId]);

  const report = qualityDashboard?.reports?.find(r => r.report_id === reportId) ?? null;
  const loading = !qualityDashboard && !qualityStatus;

  return { report, loading, error: null, reportId };
}
