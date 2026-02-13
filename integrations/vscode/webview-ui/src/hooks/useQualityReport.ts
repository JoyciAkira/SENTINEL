import { useEffect, useState, useMemo } from "react";
import { useVSCodeAPI } from "./useVSCodeAPI";
import { useMCPMessages } from "./useMCPMessages";

export interface QualityDimension {
  metric: string;
  value: number;
  threshold: number;
  gate: "hard" | "soft";
  result: "pass" | "fail";
}

export interface QualityGate {
  id: string;
  name: string;
  status: "pass" | "fail" | "pending";
  message?: string;
}

export interface QualityReport {
  schema_version: string;
  report_id: string;
  run_id: string;
  module_id: string;
  scores: QualityDimension[];
  overall: "pass" | "fail";
  linked_artifact_ids: string[];
  metadata: {
    llm_provider: string;
    model: string;
    evaluated_at: string;
    evaluation_duration_ms: number;
  };
}

/**
 * Hook for accessing quality reports from Sentinel
 */
export function useQualityReport() {
  const [report, setReport] = useState<QualityReport | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const vscodeApi = useVSCodeAPI();
  const { sendMessage, onMessage } = useMCPMessages();

  useEffect(() => {
    // Request quality report on mount
    const requestReport = () => {
      sendMessage({
        type: "get_quality_report",
      });
    };

    const handleResponse = (message: any) => {
      if (message.type === "quality_report") {
        setReport(message.data);
        setLoading(false);
      } else if (message.type === "error") {
        setError(message.data || "Failed to load quality report");
        setLoading(false);
      }
    };

    onMessage("quality_report", handleResponse);

    // Also listen for updates
    onMessage("quality_report_update", (message: any) => {
      if (message.data) {
        setReport(message.data);
      }
    });

    requestReport();

    return () => {
      // Cleanup listeners
    };
  }, [vscodeApi, onMessage]);

  // Calculate derived metrics
  const overallScore = useMemo(() => {
    if (!report) return 0;
    const passCount = report.scores.filter(s => s.result === "pass").length;
    return Math.round((passCount / report.scores.length) * 100);
  }, [report]);

  const passRate = useMemo(() => {
    if (!report) return 0;
    const passed = report.scores.filter(s => s.result === "pass").length;
    return (passed / report.scores.length) * 100;
  }, [report]);

  return {
    report,
    loading,
    error,
    overallScore,
    passRate,
    refresh: () => requestReport(),
  };
}
