import React, { useRef, useEffect } from 'react';
import { useStore } from '../../state/store';
import MessageBubble from './MessageBubble';

export default function MessageList() {
    const messages = useStore((s) => s.messages);
    const bottomRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
    }, [messages]);

    if (messages.length === 0) {
        return (
            <div style={{
                flex: 1,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--vscode-descriptionForeground)',
                fontSize: '13px',
                padding: '20px',
                textAlign: 'center',
            }}>
                Ask Sentinel to validate actions, check alignment, or plan your next steps.
            </div>
        );
    }

    return (
        <div style={{
            flex: 1,
            overflowY: 'auto',
            padding: '8px 0',
        }}>
            {messages.map((msg) => (
                <MessageBubble key={msg.id} message={msg} />
            ))}
            <div ref={bottomRef} />
        </div>
    );
}
