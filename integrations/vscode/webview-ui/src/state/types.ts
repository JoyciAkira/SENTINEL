export interface ChatMessage {
    id: string;
    role: 'user' | 'assistant';
    content: string; // The visible response
    appSpec?: AppSpecPayload;
    sections?: ChatSection[];
    innovation?: InnovationPayload;
    thoughtChain?: string[]; // Internal reasoning steps (NEW)
    explainability?: {
        intent_summary?: string;
        evidence?: string[];
        alignment_score?: number | null;
        reliability_healthy?: boolean | null;
        governance_pending_proposal?: string | null;
        context_provider?: string | null;
        context_policy_mode?: string | null;
        context_fallback_reason?: string | null;
    };
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
    error?: string;
    diff?: string;
}

export interface ChatSection {
    id: string;
    title: string;
    content: string;
    language?: string;
    pathHint?: string;
}

export type AppSpecFieldType = "string" | "number" | "boolean" | "date" | "enum";

export interface AppSpecField {
    name: string;
    type: AppSpecFieldType;
    required: boolean;
}

export interface AppSpecEntity {
    name: string;
    fields: AppSpecField[];
}

export interface AppSpecView {
    id: string;
    type: "dashboard" | "list" | "detail" | "form";
    title: string;
    entity?: string;
}

export interface AppSpecAction {
    id: string;
    type: "create" | "update" | "delete" | "read" | "custom";
    title: string;
    entity?: string;
    requiresApproval?: boolean;
}

export interface AppSpecPolicy {
    id: string;
    rule: string;
    level: "hard" | "soft";
}

export interface AppSpecIntegration {
    id: string;
    provider: string;
    purpose: string;
    required: boolean;
}

export interface AppSpecTest {
    id: string;
    type: "unit" | "integration" | "e2e" | "policy";
    description: string;
}

export interface AppSpecPayload {
    version: "1.0";
    app: {
        name: string;
        summary: string;
    };
    dataModel: {
        entities: AppSpecEntity[];
    };
    views: AppSpecView[];
    actions: AppSpecAction[];
    policies: AppSpecPolicy[];
    integrations: AppSpecIntegration[];
    tests: AppSpecTest[];
    meta: {
        source: "heuristic_v1" | "assistant_payload";
        confidence: number;
        generated_at: string;
        prompt_excerpt?: string;
        validation?: {
            status: "strict" | "fallback";
            issues?: string[];
        };
    };
}

export interface InnovationPlan {
    id?: string;
    title?: string;
    strategy?: string;
    risk?: string;
}

export interface InnovationPayload {
    version?: number;
    constitutional_spec_hash?: string;
    counterfactual_hash?: string;
    policy_simulation_hash?: string;
    constitutional_spec_path?: string | null;
    team_memory_graph_path?: string | null;
    constitutional_spec?: {
        objective?: string;
        constraints?: string[];
        invariants?: string[];
    };
    counterfactual_plans?: {
        recommended_plan_id?: string;
        recommended_reason?: string;
        plans?: InnovationPlan[];
    };
    policy_simulation?: {
        available?: boolean;
        reason?: string;
        modes?: Array<{
            mode?: string;
            healthy?: boolean;
            violations?: string[];
        }>;
    };
    team_memory_graph?: {
        node_count?: number;
        edge_count?: number;
        graph_hash?: string;
        signature_scheme?: string;
    };
    replay_ledger?: {
        path?: string | null;
        entry?: {
            turn_id?: string;
            strict_goal_execution?: boolean;
        };
    };
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
    world_model?: {
        where_we_are?: unknown;
        where_we_must_go?: unknown;
        deterministic_drift?: unknown;
        required_missing_now?: unknown;
        how_enforced?: {
            manifold_version?: number;
            manifold_integrity_hash?: string;
        };
    } | null;
}

export interface PolicyActionState {
    kind: string;
    ok: boolean;
    message: string;
    timestamp: number;
}

export interface TimelineEventState {
    id: string;
    turnId?: string;
    stage: 'received' | 'plan' | 'tool' | 'stream' | 'approval' | 'result' | 'error' | 'cancel';
    title: string;
    detail?: string;
    timestamp: number;
}

export interface GoalNodeState {
    id: string;
    description: string;
    status: string;
}

export interface RuntimeCapabilitiesState {
    tool_count: number;
    tools: string[];
    server_name: string;
    server_version: string;
    connected: boolean;
}

export interface AugmentSettingsState {
    enabled: boolean;
    mode: "disabled" | "internal_only" | "byo_customer";
    enforceByo: boolean;
}

export interface QualityStatusState {
    ok: boolean;
    latest: {
        run_id?: string;
        duration_sec?: number;
        overall_ok?: boolean;
        kpi?: {
            total_tests?: number;
            passed?: number;
            failed?: number;
            pass_rate?: number;
        };
        path?: string;
    } | null;
    message?: string;
}

export interface UiKpiHistoryState {
    sample_count: number;
    latest: {
        turns_total: number;
        natural_language_turns: number;
        slash_turns: number;
        auto_routed_turns: number;
        auto_route_rate: number;
        median_prompt_to_plan_ms: number;
        pending_approvals: number;
        approval_rate: number;
        timestamp: number;
    } | null;
    summary_7d: {
        samples: number;
        turns_total: number;
        auto_route_rate: number;
        median_prompt_to_plan_ms: number;
        approval_rate: number;
    };
    summary_30d: {
        samples: number;
        turns_total: number;
        auto_route_rate: number;
        median_prompt_to_plan_ms: number;
        approval_rate: number;
    };
    series_14d: Array<{
        date: string;
        auto_route_rate: number;
    }>;
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
    timeline: TimelineEventState[];
    goals: GoalNodeState[];
    runtimeCapabilities: RuntimeCapabilitiesState | null;
    augmentSettings: AugmentSettingsState;
    qualityStatus: QualityStatusState | null;
    uiKpiHistory: UiKpiHistoryState | null;
    goalsCollapsed: boolean;
    inputText: string;

    // Actions
    setConnected: (connected: boolean) => void;
    addMessage: (msg: ChatMessage) => void;
    updateLastAssistant: (
        content: string,
        thoughts?: string[],
        explainability?: ChatMessage["explainability"],
        sections?: ChatSection[],
        innovation?: InnovationPayload,
        fileOperations?: FileOperation[],
        appSpec?: AppSpecPayload,
    ) => void;
    updateFileOperationApproval: (
        messageId: string,
        path: string,
        approved: boolean,
        error?: string,
    ) => void;
    appendToolCall: (messageId: string, tool: ToolCallInfo) => void;
    setAlignment: (alignment: AlignmentState) => void;
    setReliability: (
        reliability: ReliabilityState,
        thresholds: ReliabilityThresholdsState,
        slo: ReliabilitySloState,
    ) => void;
    setGovernance: (governance: GovernanceState) => void;
    setPolicyAction: (action: PolicyActionState) => void;
    addTimelineEvent: (event: TimelineEventState) => void;
    clearTimeline: () => void;
    setGoals: (goals: GoalNodeState[]) => void;
    setRuntimeCapabilities: (capabilities: RuntimeCapabilitiesState) => void;
    setAugmentSettings: (settings: AugmentSettingsState) => void;
    setQualityStatus: (quality: QualityStatusState) => void;
    setUiKpiHistory: (history: UiKpiHistoryState) => void;
    toggleGoalsCollapsed: () => void;
    setInputText: (text: string) => void;
    clearMessages: () => void;
}
