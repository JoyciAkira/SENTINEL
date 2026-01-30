import React, { useState } from 'react';
import type { ToolCallInfo } from '../../state/types';

interface Props {
    tool: ToolCallInfo;
}

const STATUS_LABELS: Record<string, string> = {
    pending: '\u23F3 Running...',
    success: '\u2705',
    error: '\u274C Error',
};

export default function ToolCallCard({ tool }: Props) {
    const [expanded, setExpanded] = useState(false);

    return (
        <div style={{
            margin: '6px 0',
            border: '1px solid var(--vscode-panel-border)',
            borderRadius: '4px',
            overflow: 'hidden',
            fontSize: '12px',
        }}>
            <div
                onClick={() => setExpanded(!expanded)}
                style={{
                    padding: '6px 10px',
                    backgroundColor: 'var(--vscode-editor-background)',
                    cursor: 'pointer',
                    display: 'flex',
                    justifyContent: 'space-between',
                    alignItems: 'center',
                    userSelect: 'none',
                }}
            >
                <span>
                    <strong>Tool:</strong> {tool.name}
                </span>
                <span>{STATUS_LABELS[tool.status] ?? tool.status}</span>
            </div>
            {expanded && (
                <div style={{
                    padding: '8px 10px',
                    backgroundColor: 'var(--vscode-input-background)',
                    borderTop: '1px solid var(--vscode-panel-border)',
                }}>
                    <div style={{ marginBottom: '4px' }}>
                        <strong>Arguments:</strong>
                        <pre style={{
                            margin: '4px 0',
                            whiteSpace: 'pre-wrap',
                            wordBreak: 'break-word',
                        }}>
                            {JSON.stringify(tool.arguments, null, 2)}
                        </pre>
                    </div>
                    {tool.result && (
                        <div>
                            <strong>Result:</strong>
                            <pre style={{
                                margin: '4px 0',
                                whiteSpace: 'pre-wrap',
                                wordBreak: 'break-word',
                            }}>
                                {tool.result}
                            </pre>
                        </div>
                    )}
                </div>
            )}
        </div>
    );
}
