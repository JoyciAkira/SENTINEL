import React from 'react';
import type { ChatMessage } from '../../state/types';
import { renderMarkdown } from '../../utils/markdown';
import ToolCallCard from './ToolCallCard';
import FileApproval from '../Actions/FileApproval';

interface Props {
    message: ChatMessage;
}

export default function MessageBubble({ message }: Props) {
    const isUser = message.role === 'user';

    return (
        <div style={{
            display: 'flex',
            justifyContent: isUser ? 'flex-end' : 'flex-start',
            padding: '4px 12px',
        }}>
            <div style={{
                maxWidth: isUser ? '85%' : '100%',
                padding: '8px 12px',
                borderRadius: '8px',
                backgroundColor: isUser
                    ? 'var(--vscode-button-background)'
                    : 'var(--vscode-editor-background)',
                color: isUser
                    ? 'var(--vscode-button-foreground)'
                    : 'var(--vscode-foreground)',
                fontSize: '13px',
                lineHeight: '1.5',
                wordBreak: 'break-word',
            }}>
                {!isUser && (
                    <div style={{
                        fontSize: '11px',
                        fontWeight: 600,
                        marginBottom: '4px',
                        color: 'var(--vscode-textLink-foreground)',
                    }}>
                        Sentinel
                    </div>
                )}

                <div
                    dangerouslySetInnerHTML={{ __html: renderMarkdown(message.content) }}
                    style={{ overflow: 'hidden' }}
                />

                {message.toolCalls?.map((tool, i) => (
                    <ToolCallCard key={i} tool={tool} />
                ))}

                {message.fileOperations?.map((op, i) => (
                    <FileApproval key={i} operation={op} messageId={message.id} />
                ))}

                {message.streaming && (
                    <span style={{
                        display: 'inline-block',
                        width: '8px',
                        height: '16px',
                        backgroundColor: 'var(--vscode-textLink-foreground)',
                        marginLeft: '2px',
                        animation: 'blink 1s step-end infinite',
                    }} />
                )}
            </div>
        </div>
    );
}
