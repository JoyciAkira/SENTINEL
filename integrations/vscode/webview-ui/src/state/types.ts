export interface ChatMessage {
    id: string;
    role: 'user' | 'assistant';
    content: string; // The visible response
    thoughtChain?: string[]; // Internal reasoning steps (NEW)
    timestamp: number;
    toolCalls?: ToolCallInfo[];
    fileOperations?: FileOperation[];
    streaming?: boolean;
}

export interface ToolCallInfo {
    name: string;
    arguments: Record<string, unknown>;
    result?: string;
    status: 'pending' | 'success' | 'error';
}

export interface FileOperation {
    path: string;
    type: 'create' | 'edit' | 'delete';
    linesAdded?: number;
    linesRemoved?: number;
    approved?: boolean;
    diff?: string;
}

export interface AlignmentState {
    score: number;
    confidence: number;
    status: string;
    trend: number; // positive = improving, negative = degrading
}

export interface GoalNodeState {
    id: string;
    description: string;
    status: string;
}

export interface AppState {
    connected: boolean;
    messages: ChatMessage[];
    alignment: AlignmentState | null;
    goals: GoalNodeState[];
    goalsCollapsed: boolean;
    inputText: string;

    // Actions
    setConnected: (connected: boolean) => void;
    addMessage: (msg: ChatMessage) => void;
    updateLastAssistant: (content: string, thoughts?: string[]) => void;
    appendToolCall: (messageId: string, tool: ToolCallInfo) => void;
    setAlignment: (alignment: AlignmentState) => void;
    setGoals: (goals: GoalNodeState[]) => void;
    toggleGoalsCollapsed: () => void;
    setInputText: (text: string) => void;
    clearMessages: () => void;
}
