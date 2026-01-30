import { create } from 'zustand';
import type { AppState, ChatMessage, ToolCallInfo, AlignmentState, GoalNodeState } from './types';

export const useStore = create<AppState>((set) => ({
    connected: false,
    messages: [],
    alignment: null,
    goals: [],
    goalsCollapsed: true,
    inputText: '',

    setConnected: (connected: boolean) => set({ connected }),

    addMessage: (msg: ChatMessage) =>
        set((state) => ({ messages: [...state.messages, msg] })),

    updateLastAssistant: (content: string) =>
        set((state) => {
            const msgs = [...state.messages];
            for (let i = msgs.length - 1; i >= 0; i--) {
                if (msgs[i].role === 'assistant') {
                    msgs[i] = { ...msgs[i], content, streaming: false };
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

    setGoals: (goals: GoalNodeState[]) => set({ goals }),

    toggleGoalsCollapsed: () =>
        set((state) => ({ goalsCollapsed: !state.goalsCollapsed })),

    setInputText: (text: string) => set({ inputText: text }),

    clearMessages: () => set({ messages: [] }),
}));
