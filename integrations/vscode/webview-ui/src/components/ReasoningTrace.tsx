/**
 * ReasoningTrace Component
 * 
 * Visualizes LLM reasoning traces in the SENTINEL VSCode extension.
 * Shows step-by-step analysis of why search results are relevant.
 */

import React from 'react';

export interface ReasoningStep {
  action: string;
  observation: string;
  decision: string;
  node_id?: string;
}

export interface ReasoningTraceData {
  query: string;
  steps: ReasoningStep[];
  confidence: number;
  rationale: string;
}

interface ReasoningTraceProps {
  trace: ReasoningTraceData | null;
  isLoading?: boolean;
  error?: string | null;
}

export const ReasoningTrace: React.FC<ReasoningTraceProps> = ({
  trace,
  isLoading,
  error,
}) => {
  if (isLoading) {
    return (
      <div className="reasoning-trace-loading">
        <div className="loading-spinner"></div>
        <span>Generating reasoning trace...</span>
      </div>
    );
  }

  if (error) {
    return (
      <div className="reasoning-trace-error">
        <span className="error-icon">⚠️</span>
        <span>{error}</span>
      </div>
    );
  }

  if (!trace) {
    return (
      <div className="reasoning-trace-empty">
        <span className="empty-icon">🧠</span>
        <span>No reasoning trace available</span>
      </div>
    );
  }

  return (
    <div className="reasoning-trace-container">
      <div className="reasoning-trace-header">
        <h3>🧠 Reasoning Trace</h3>
        <span className="confidence-badge">
          Confidence: {(trace.confidence * 100).toFixed(0)}%
        </span>
      </div>

      <div className="reasoning-query">
        <strong>Query:</strong> {trace.query}
      </div>

      <div className="reasoning-steps">
        {trace.steps.map((step, index) => (
          <div key={index} className="reasoning-step">
            <div className="step-header">
              <span className="step-number">{index + 1}</span>
              <span className="step-action">{step.action}</span>
            </div>
            
            <div className="step-content">
              <div className="step-observation">
                <span className="label">Observation:</span>
                <span className="value">{step.observation}</span>
              </div>
              
              <div className="step-decision">
                <span className="label">Decision:</span>
                <span className="value">{step.decision}</span>
              </div>
              
              {step.node_id && (
                <div className="step-node">
                  <span className="label">Node:</span>
                  <code className="value">{step.node_id}</code>
                </div>
              )}
            </div>
          </div>
        ))}
      </div>

      <div className="reasoning-rationale">
        <strong>Rationale:</strong>
        <p>{trace.rationale}</p>
      </div>
    </div>
  );
};

export default ReasoningTrace;