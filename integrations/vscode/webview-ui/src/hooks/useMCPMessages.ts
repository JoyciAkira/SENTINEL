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
    const setReliability = useStore((s) => s.setReliability);
    const setGovernance = useStore((s) => s.setGovernance);
    const setPolicyAction = useStore((s) => s.setPolicyAction);
    const addTimelineEvent = useStore((s) => s.addTimelineEvent);
    const clearTimeline = useStore((s) => s.clearTimeline);
    const setGoals = useStore((s) => s.setGoals);
    const setRuntimeCapabilities = useStore((s) => s.setRuntimeCapabilities);
    const setAugmentSettings = useStore((s) => s.setAugmentSettings);
    const setQualityStatus = useStore((s) => s.setQualityStatus);

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

                case 'timelineReset':
                    clearTimeline();
                    break;

                case 'timelineEvent':
                    addTimelineEvent({
                        id: msg.id ?? crypto.randomUUID(),
                        turnId: msg.turnId,
                        stage: msg.stage ?? 'result',
                        title: msg.title ?? 'Timeline event',
                        detail: msg.detail,
                        timestamp: msg.timestamp ?? Date.now(),
                    });
                    break;

                case 'chatResponse':
                    if (msg.streaming) {
                        addMessage({
                            id: msg.id ?? crypto.randomUUID(),
                            role: 'assistant',
                            content: msg.content ?? '',
                            timestamp: Date.now(),
                            toolCalls: msg.toolCalls,
                            streaming: true,
                        });
                    } else {
                        const existing = useStore
                            .getState()
                            .messages.some((m) => m.id === msg.id && m.role === 'assistant');
                        if (existing) {
                            updateLastAssistant(msg.content ?? '', msg.thoughtChain, msg.explainability);
                        } else {
                            addMessage({
                                id: msg.id ?? crypto.randomUUID(),
                                role: 'assistant',
                                content: msg.content ?? '',
                                timestamp: Date.now(),
                                toolCalls: msg.toolCalls,
                                thoughtChain: msg.thoughtChain,
                                explainability: msg.explainability,
                                streaming: false,
                            });
                        }
                    }
                    break;

                case 'chatStreaming':
                    // Update the last assistant message content
                    updateLastAssistant(msg.content ?? '');
                    break;

                case 'chatStreamingStopped':
                    {
                        const msgs = useStore.getState().messages;
                        const last = msgs.length > 0 ? msgs[msgs.length - 1] : undefined;
                        updateLastAssistant(last?.content ?? '');
                    }
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

                case 'runtimeCapabilities':
                    if (msg.capabilities) {
                        setRuntimeCapabilities(msg.capabilities);
                    }
                    break;

                case 'augmentSettingsUpdate':
                    if (msg.settings) {
                        setAugmentSettings(msg.settings);
                    }
                    break;

                case 'reliabilityUpdate':
                    if (msg.reliability && msg.reliability_thresholds && msg.reliability_slo) {
                        setReliability(msg.reliability, msg.reliability_thresholds, msg.reliability_slo);
                    }
                    break;

                case 'governanceUpdate':
                    if (msg.governance) {
                        setGovernance(msg.governance);
                    }
                    break;

                case 'policyActionResult':
                    setPolicyAction({
                        kind: msg.kind ?? 'unknown',
                        ok: Boolean(msg.ok),
                        message: msg.message ?? '',
                        timestamp: Date.now(),
                    });
                    break;

                case 'qualityUpdate':
                    if (msg.quality) {
                        setQualityStatus(msg.quality);
                    }
                    break;
            }
        };

        window.addEventListener('message', handler);
        return () => window.removeEventListener('message', handler);
    }, [
        setConnected,
        addMessage,
        updateLastAssistant,
        appendToolCall,
        setAlignment,
        setReliability,
        setGovernance,
        setPolicyAction,
        addTimelineEvent,
        clearTimeline,
        setGoals,
        setRuntimeCapabilities,
        setAugmentSettings,
        setQualityStatus,
    ]);
}
