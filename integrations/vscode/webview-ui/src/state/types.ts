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

export interface ReliabilityThresholdsState {
    min_task_success_rate: number;
    min_no_regression_rate: number;
    max_rollback_rate: number;
    max_mean_time_to_recover_ms: number;
    max_invariant_violation_rate: number;
}

export interface ReliabilitySloState {
    healthy: boolean;
    violations: string[];
}

export interface ReliabilityState {
    task_success_rate: number;
    no_regression_rate: number;
    rollback_rate: number;
    avg_time_to_recover_ms: number;
    invariant_violation_rate: number;
}

export interface GovernanceProposalState {
    id: string;
    rationale: string;
    created_at: string;
    status: string;
}

export interface GovernanceState {
    required_dependencies: string[];
    allowed_dependencies: string[];
    required_frameworks: string[];
    allowed_frameworks: string[];
    allowed_endpoints: Record<string, string>;
    allowed_ports: number[];
    pending_proposal: GovernanceProposalState | null;
    history_size: number;
}

export interface PolicyActionState {
    kind: string;
    ok: boolean;
    message: string;
    timestamp: number;
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
    reliability: ReliabilityState | null;
    reliabilityThresholds: ReliabilityThresholdsState | null;
    reliabilitySlo: ReliabilitySloState | null;
    governance: GovernanceState | null;
    policyAction: PolicyActionState | null;
    goals: GoalNodeState[];
    goalsCollapsed: boolean;
    inputText: string;

    // Actions
    setConnected: (connected: boolean) => void;
    addMessage: (msg: ChatMessage) => void;
    updateLastAssistant: (content: string, thoughts?: string[]) => void;
    appendToolCall: (messageId: string, tool: ToolCallInfo) => void;
    setAlignment: (alignment: AlignmentState) => void;
    setReliability: (
        reliability: ReliabilityState,
        thresholds: ReliabilityThresholdsState,
        slo: ReliabilitySloState,
    ) => void;
    setGovernance: (governance: GovernanceState) => void;
    setPolicyAction: (action: PolicyActionState) => void;
    setGoals: (goals: GoalNodeState[]) => void;
    toggleGoalsCollapsed: () => void;
    setInputText: (text: string) => void;
    clearMessages: () => void;
}
