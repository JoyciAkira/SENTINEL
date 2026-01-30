import { useEffect } from 'react';
import { useStore } from '../state/store';

interface VSCodeAPI {
    postMessage(message: unknown): void;
    getState(): unknown;
    setState(state: unknown): void;
}

/**
 * Listens for postMessage events from the extension host
 * and updates Zustand store accordingly.
 */
export function useMCPMessages(vscodeApi: VSCodeAPI): void {
    const setConnected = useStore((s) => s.setConnected);
    const addMessage = useStore((s) => s.addMessage);
    const updateLastAssistant = useStore((s) => s.updateLastAssistant);
    const appendToolCall = useStore((s) => s.appendToolCall);
    const setAlignment = useStore((s) => s.setAlignment);
    const setGoals = useStore((s) => s.setGoals);

    useEffect(() => {
        const handler = (event: MessageEvent) => {
            const msg = event.data;
            if (!msg || !msg.type) return;

            switch (msg.type) {
                case 'connected':
                    setConnected(true);
                    break;

                case 'disconnected':
                    setConnected(false);
                    break;

                case 'chatResponse':
                    addMessage({
                        id: msg.id ?? crypto.randomUUID(),
                        role: 'assistant',
                        content: msg.content,
                        timestamp: Date.now(),
                        toolCalls: msg.toolCalls,
                        streaming: false,
                    });
                    break;

                case 'chatStreaming':
                    // Update the last assistant message content
                    updateLastAssistant(msg.content);
                    break;

                case 'toolCall':
                    appendToolCall(msg.messageId, {
                        name: msg.name,
                        arguments: msg.arguments ?? {},
                        result: msg.result,
                        status: msg.status ?? 'pending',
                    });
                    break;

                case 'alignmentUpdate':
                    setAlignment({
                        score: msg.score,
                        confidence: msg.confidence,
                        status: msg.status,
                        trend: msg.trend ?? 0,
                    });
                    break;

                case 'goalsUpdate':
                    setGoals(msg.goals ?? []);
                    break;
            }
        };

        window.addEventListener('message', handler);
        return () => window.removeEventListener('message', handler);
    }, [setConnected, addMessage, updateLastAssistant, appendToolCall, setAlignment, setGoals]);
}
