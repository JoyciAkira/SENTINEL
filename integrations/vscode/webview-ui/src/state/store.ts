import { create } from 'zustand';
import type { AppState, ChatMessage, ToolCallInfo, AlignmentState, GoalNodeState } from './types';

export const useStore = create<AppState>((set) => ({
    connected: false,
    messages: [],
    alignment: null,
    reliability: null,
    reliabilityThresholds: null,
    reliabilitySlo: null,
    governance: null,
    policyAction: null,
    timeline: [],
    goals: [],
    runtimeCapabilities: null,
    augmentSettings: {
        enabled: false,
        mode: "disabled",
        enforceByo: true,
    },
    goalsCollapsed: true,
    inputText: '',

    setConnected: (connected: boolean) => set({ connected }),

    addMessage: (msg: ChatMessage) =>
        set((state) => ({ messages: [...state.messages, msg] })),

    updateLastAssistant: (content: string, thoughts?: string[], explainability?) =>
        set((state) => {
            const msgs = [...state.messages];
            for (let i = msgs.length - 1; i >= 0; i--) {
                if (msgs[i].role === 'assistant') {
                    msgs[i] = { 
                        ...msgs[i], 
                        content, 
                        thoughtChain: thoughts || msgs[i].thoughtChain, // Preserve existing thoughts if not provided
                        explainability: explainability || msgs[i].explainability,
                        streaming: false 
                    };
                    break;
                }
            }
            return { messages: msgs };
        }),

    appendToolCall: (messageId: string, tool: ToolCallInfo) =>
        set((state) => {
            const msgs = state.messages.map((m) => {
                if (m.id === messageId) {
                    return {
                        ...m,
                        toolCalls: [...(m.toolCalls ?? []), tool],
                    };
                }
                return m;
            });
            return { messages: msgs };
        }),

    setAlignment: (alignment: AlignmentState) => set({ alignment }),

    setReliability: (reliability, reliabilityThresholds, reliabilitySlo) =>
        set({ reliability, reliabilityThresholds, reliabilitySlo }),

    setGovernance: (governance) => set({ governance }),

    setPolicyAction: (policyAction) => set({ policyAction }),

    addTimelineEvent: (event) =>
        set((state) => ({
            timeline: [...state.timeline, event].slice(-300),
        })),

    clearTimeline: () => set({ timeline: [] }),

    setGoals: (goals: GoalNodeState[]) => set({ goals }),

    setRuntimeCapabilities: (runtimeCapabilities) => set({ runtimeCapabilities }),

    setAugmentSettings: (augmentSettings) => set({ augmentSettings }),

    toggleGoalsCollapsed: () =>
        set((state) => ({ goalsCollapsed: !state.goalsCollapsed })),

    setInputText: (text: string) => set({ inputText: text }),

    clearMessages: () => set({ messages: [] }),
}));
