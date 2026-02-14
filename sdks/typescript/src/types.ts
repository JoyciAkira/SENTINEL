export enum InvariantSeverity {
    Warning = "Warning",
    Error = "Error",
    Critical = "Critical"
}

export enum PredicateType {
    AlwaysTrue = "AlwaysTrue",
    AlwaysFalse = "AlwaysFalse",
    FileExists = "FileExists",
    TestsPassing = "TestsPassing",
    Custom = "Custom"
}

export interface Predicate {
    type: PredicateType;
    params?: Record<string, any>;
}

export interface Invariant {
    id: string; // UUID
    description: string;
    severity: InvariantSeverity;
    predicate: Predicate;
}

export interface Intent {
    description: string;
    constraints: string[];
    expected_outcomes: string[];
    target_platform?: string;
    languages: string[];
    frameworks: string[];
    infrastructure_map: Record<string, string>;
}

export enum GoalStatus {
    Pending = "Pending",
    Ready = "Ready",
    InProgress = "InProgress",
    Validating = "Validating",
    Completed = "Completed",
    Blocked = "Blocked",
    Failed = "Failed",
    Deprecated = "Deprecated"
}

export interface Goal {
    id: string; // UUID
    description: string;
    parent_id?: string;
    dependencies: string[];
    status: GoalStatus;
    success_criteria: Predicate[];
    value_to_root: number;
}

export interface ManifoldVersion {
    version: number;
    timestamp: string; // ISO 8601
    hash: string;
    change_description: string;
}

export interface GoalManifold {
    root_intent: Intent;
    sensitivity: number;
    goals: Goal[];
    invariants: Invariant[];
    created_at: string;
    updated_at: string;
    integrity_hash: string;
    version_history: ManifoldVersion[];
}
