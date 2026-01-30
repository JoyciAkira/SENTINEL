// Types mirroring Rust Sentinel types for the VS Code extension

export interface GoalManifold {
    root_intent: Intent;
    sensitivity: number;
    file_locks: Record<string, string>;
    handover_log: HandoverNote[];
    goal_dag: GoalDag;
    invariants: Invariant[];
    integrity_hash: string;
    version_history: ManifoldVersion[];
    peer_count?: number;
    consensus_active?: boolean;
}

export interface Intent {
    description: string;
    constraints: string[];
    success_criteria: string[];
}

export interface GoalDag {
    nodes: Record<string, GoalNode>;
}

export interface GoalNode {
    id: string;
    description: string;
    status: GoalStatus;
    dependencies: string[];
    value_to_root: number;
}

export type GoalStatus =
    | 'Pending'
    | 'Ready'
    | 'InProgress'
    | 'Validating'
    | 'Completed'
    | 'Blocked'
    | 'Failed'
    | 'Deprecated';

export interface HandoverNote {
    agent_id: string;
    content: string;
    warnings: string[];
    timestamp: string;
}

export interface Invariant {
    description: string;
    constraint_type: string;
}

export interface ManifoldVersion {
    version: number;
    timestamp: string;
    changes: string;
}

export interface AlignmentReport {
    score: number;
    confidence: number;
    violations: AlignmentViolation[];
    status: 'OPTIMAL' | 'ACCEPTABLE' | 'DEVIATED' | 'CRITICAL';
}

export interface AlignmentViolation {
    description: string;
    severity: number;
    goal_id?: string;
}

export interface ValidationResult {
    alignment_score: number;
    deviation_probability: number;
    risk_level: string;
    approved: boolean;
    rationale: string;
    alternatives?: string[];
}

export interface SecurityScanResult {
    risk_score: number;
    is_safe: boolean;
    threats: SecurityThreat[];
}

export interface SecurityThreat {
    description: string;
    severity: number;
    pattern: string;
}

export interface StrategyRecommendation {
    confidence: number;
    patterns: SuccessPattern[];
    pitfalls: string[];
}

export interface SuccessPattern {
    id: string;
    name: string;
    description: string;
    success_rate: number;
    applicable_to_goal_types: string[];
}

export interface CognitiveMap {
    goals: CognitiveGoal[];
    current_focus?: string;
    cognitive_mode: string;
}

export interface CognitiveGoal {
    id: string;
    description: string;
    status: GoalStatus;
    depth: number;
    children: string[];
}

export interface EnforcementRule {
    description: string;
    constraint_type: string;
    active: boolean;
}

export interface StatusReport {
    manifold: GoalManifold;
    external?: ExternalReport;
}

export interface ExternalReport {
    risk_level: number;
    alerts: string[];
}

// MCP Protocol types
export interface McpToolCallResult {
    content: McpContent[];
    isError?: boolean;
}

export interface McpContent {
    type: 'text';
    text: string;
}

// Chat types
export interface ChatMessage {
    id: string;
    role: 'user' | 'assistant';
    content: string;
    timestamp: number;
    toolCalls?: ToolCallInfo[];
    fileOperations?: FileOperation[];
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
