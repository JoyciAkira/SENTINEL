import React, { useRef, useCallback } from 'react';
import { useStore } from '../../state/store';
import { useVSCodeAPI } from '../../hooks/useVSCodeAPI';
import { colors } from '../../utils/theme';

export default function ChatInput() {
    const inputText = useStore((s) => s.inputText);
    const setInputText = useStore((s) => s.setInputText);
    const addMessage = useStore((s) => s.addMessage);
    const connected = useStore((s) => s.connected);
    const vscode = useVSCodeAPI();
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    const send = useCallback(() => {
        const text = inputText.trim();
        if (!text || !connected) return;

        addMessage({
            id: crypto.randomUUID(),
            role: 'user',
            content: text,
            timestamp: Date.now(),
        });

        vscode.postMessage({
            type: 'chatMessage',
            text,
        });

        setInputText('');

        // Reset textarea height
        if (textareaRef.current) {
            textareaRef.current.style.height = 'auto';
        }
    }, [inputText, connected, addMessage, setInputText, vscode]);

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter' && !e.shiftKey) {
            e.preventDefault();
            send();
        }
    };

    const handleInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        setInputText(e.target.value);
        // Auto-grow up to 5 lines
        const el = e.target;
        el.style.height = 'auto';
        el.style.height = Math.min(el.scrollHeight, 100) + 'px';
    };

    return (
        <div style={{
            padding: '8px 12px',
            borderTop: `1px solid var(--vscode-panel-border)`,
            flexShrink: 0,
        }}>
            <div style={{
                display: 'flex',
                gap: '8px',
                alignItems: 'flex-end',
            }}>
                <textarea
                    ref={textareaRef}
                    value={inputText}
                    onChange={handleInput}
                    onKeyDown={handleKeyDown}
                    placeholder={connected ? 'Ask Sentinel...' : 'Not connected'}
                    disabled={!connected}
                    rows={1}
                    style={{
                        flex: 1,
                        resize: 'none',
                        padding: '8px',
                        border: `1px solid ${colors.inputBorder}`,
                        borderRadius: '4px',
                        backgroundColor: colors.inputBg,
                        color: colors.inputFg,
                        fontFamily: 'var(--vscode-font-family)',
                        fontSize: '13px',
                        lineHeight: '1.4',
                        outline: 'none',
                        maxHeight: '100px',
                        overflow: 'auto',
                    }}
                />
                <button
                    onClick={send}
                    disabled={!connected || !inputText.trim()}
                    style={{
                        padding: '8px 14px',
                        border: 'none',
                        borderRadius: '4px',
                        backgroundColor: colors.buttonBg,
                        color: colors.buttonFg,
                        cursor: connected && inputText.trim() ? 'pointer' : 'default',
                        opacity: connected && inputText.trim() ? 1 : 0.5,
                        fontSize: '13px',
                        flexShrink: 0,
                    }}
                >
                    Send
                </button>
            </div>
        </div>
    );
}
