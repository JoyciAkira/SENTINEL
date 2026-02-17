import React, { Component, ErrorInfo, ReactNode } from "react";

interface Props {
  /** Content to protect */
  children: ReactNode;
  /** Optional label used in the error message (e.g. "Forge", "Network") */
  label?: string;
  /** Optional custom fallback UI */
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

/**
 * ErrorBoundary — catches unhandled render errors in a subtree and shows
 * a minimal recovery UI instead of crashing the entire webview.
 *
 * Usage:
 *   <ErrorBoundary label="Forge">
 *     <SplitAgentPanel />
 *   </ErrorBoundary>
 */
export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, info: ErrorInfo): void {
    // Log to console so VSCode Developer Tools can capture it
    console.error(`[Sentinel ErrorBoundary] ${this.props.label ?? "Panel"} crashed:`, error, info.componentStack);
  }

  private handleReset = (): void => {
    this.setState({ hasError: false, error: null });
  };

  render(): ReactNode {
    if (!this.state.hasError) {
      return this.props.children;
    }

    if (this.props.fallback) {
      return this.props.fallback;
    }

    const label = this.props.label ?? "Panel";
    const message = this.state.error?.message ?? "Unknown error";

    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          padding: "24px 16px",
          gap: 12,
          textAlign: "center",
          color: "var(--vscode-errorForeground, #f48771)",
          minHeight: 120,
        }}
      >
        <span style={{ fontSize: 20 }}>⚠️</span>
        <p style={{ margin: 0, fontWeight: 600, fontSize: 13 }}>
          {label} encountered a render error
        </p>
        <p
          style={{
            margin: 0,
            fontSize: 11,
            color: "var(--vscode-descriptionForeground, #8a999e)",
            maxWidth: 320,
            wordBreak: "break-word",
          }}
        >
          {message}
        </p>
        <button
          onClick={this.handleReset}
          style={{
            marginTop: 8,
            padding: "4px 12px",
            fontSize: 11,
            cursor: "pointer",
            background: "var(--vscode-button-background, #0e639c)",
            color: "var(--vscode-button-foreground, #fff)",
            border: "none",
            borderRadius: 3,
          }}
        >
          Retry
        </button>
      </div>
    );
  }
}

export default ErrorBoundary;
