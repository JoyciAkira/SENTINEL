import React from 'react';
import type { FileOperation } from '../../state/types';
import { useVSCodeAPI } from '../../hooks/useVSCodeAPI';
import { colors } from '../../utils/theme';

interface Props {
    operation: FileOperation;
    messageId: string;
}

export default function FileApproval({ operation, messageId }: Props) {
    const vscode = useVSCodeAPI();

    const handleApprove = () => {
        vscode.postMessage({
            type: 'fileApproval',
            messageId,
            path: operation.path,
            approved: true,
        });
    };

    const handleReject = () => {
        vscode.postMessage({
            type: 'fileApproval',
            messageId,
            path: operation.path,
            approved: false,
        });
    };

    if (operation.approved !== undefined) {
        return (
            <div style={cardStyle}>
                <span>
                    {operation.approved ? '\u2705' : '\u274C'} {operation.path}
                </span>
            </div>
        );
    }

    return (
        <div style={cardStyle}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
                <span style={{ fontWeight: 600 }}>
                    File: {operation.path}
                </span>
                <span style={{ color: colors.descriptionFg, fontSize: '11px' }}>
                    {operation.type === 'create' ? 'NEW' : operation.type.toUpperCase()}
                    {operation.linesAdded ? ` +${operation.linesAdded}` : ''}
                    {operation.linesRemoved ? ` -${operation.linesRemoved}` : ''}
                </span>
            </div>
            <div style={{ display: 'flex', gap: '8px', marginTop: '8px' }}>
                <button onClick={handleApprove} style={approveStyle}>Approve</button>
                <button onClick={handleReject} style={rejectStyle}>Reject</button>
            </div>
        </div>
    );
}

const cardStyle: React.CSSProperties = {
    margin: '6px 0',
    padding: '8px 10px',
    border: '1px solid var(--vscode-panel-border)',
    borderRadius: '4px',
    backgroundColor: 'var(--vscode-editor-background)',
    fontSize: '12px',
};

const approveStyle: React.CSSProperties = {
    padding: '4px 12px',
    border: 'none',
    borderRadius: '3px',
    backgroundColor: 'var(--vscode-button-background)',
    color: 'var(--vscode-button-foreground)',
    cursor: 'pointer',
    fontSize: '12px',
};

const rejectStyle: React.CSSProperties = {
    padding: '4px 12px',
    border: 'none',
    borderRadius: '3px',
    backgroundColor: 'var(--vscode-button-secondaryBackground)',
    color: 'var(--vscode-button-secondaryForeground)',
    cursor: 'pointer',
    fontSize: '12px',
};
