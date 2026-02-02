/**
 * SENTINEL TypeScript SDK
 * 
 * This SDK provides a type-safe interface to interact with the SENTINEL
 * goal-aligned AI coding agent system.
 * 
 * @example
 * ```typescript
 * import { SentinelClient } from '@sentinel/sdk';
 * 
 * const client = new SentinelClient('http://localhost:8080');
 * 
 * // Create a new goal manifold
 * const manifold = await client.createGoalManifold({
 *   description: 'Build a REST API',
 *   constraints: ['Use TypeScript', 'PostgreSQL']
 * });
 * 
 * // Add goals
 * await manifold.addGoal({
 *   description: 'Implement authentication',
 *   successCriteria: [
 *     { type: 'test_passing', suite: 'auth', minCoverage: 0.8 }
 *   ]
 * });
 * ```
 */

export interface SentinelConfig {
  /** Base URL of the SENTINEL daemon */
  baseUrl: string;
  /** Authentication token (if required) */
  authToken?: string;
  /** Request timeout in milliseconds */
  timeout?: number;
}

export interface Intent {
  /** Natural language description of the objective */
  description: string;
  /** User-specified constraints */
  constraints: string[];
  /** Expected outcomes */
  expectedOutcomes?: string[];
  /** Target platform */
  targetPlatform?: string;
  /** Programming languages */
  languages?: string[];
  /** Frameworks */
  frameworks?: string[];
}

export interface Goal {
  id: string;
  description: string;
  successCriteria: SuccessCriterion[];
  dependencies: string[];
  antiDependencies: string[];
  complexityEstimate: ProbabilityDistribution;
  valueToRoot: number;
  status: GoalStatus;
  parentId?: string;
  validationTests: string[];
  metadata: GoalMetadata;
  createdAt: string;
  updatedAt: string;
}

export interface SuccessCriterion {
  type: 'file_exists' | 'tests_passing' | 'api_available' | 'custom';
  params: Record<string, unknown>;
}

export interface ProbabilityDistribution {
  mean: number;
  stdDev: number;
  min: number;
  max: number;
  distributionType: 'normal' | 'uniform' | 'point';
}

export type GoalStatus = 
  | 'pending' 
  | 'ready' 
  | 'in_progress' 
  | 'validating' 
  | 'completed' 
  | 'blocked' 
  | 'failed' 
  | 'deprecated';

export interface GoalMetadata {
  tags: string[];
  notes: string;
  priority: number;
}

export interface GoalManifold {
  id: string;
  rootIntent: Intent;
  goals: Goal[];
  invariants: Invariant[];
  completionPercentage: number;
  createdAt: string;
  updatedAt: string;
}

export interface Invariant {
  id: string;
  description: string;
  severity: 'warning' | 'error' | 'critical';
}

/**
 * Main client for interacting with SENTINEL
 */
export class SentinelClient {
  private config: Required<SentinelConfig>;

  constructor(config: SentinelConfig) {
    this.config = {
      timeout: 30000,
      authToken: '',
      ...config,
    };
  }

  /**
   * Create a new goal manifold
   */
  async createGoalManifold(intent: Intent): Promise<GoalManifold> {
    const response = await this.request<GoalManifold>('/api/v1/manifolds', {
      method: 'POST',
      body: JSON.stringify(intent),
    });
    return response;
  }

  /**
   * Get a goal manifold by ID
   */
  async getGoalManifold(id: string): Promise<GoalManifold> {
    return this.request<GoalManifold>(`/api/v1/manifolds/${id}`);
  }

  /**
   * List all goal manifolds
   */
  async listGoalManifolds(): Promise<GoalManifold[]> {
    return this.request<GoalManifold[]>('/api/v1/manifolds');
  }

  /**
   * Add a goal to a manifold
   */
  async addGoal(manifoldId: string, goal: Omit<Goal, 'id'>): Promise<Goal> {
    return this.request<Goal>(`/api/v1/manifolds/${manifoldId}/goals`, {
      method: 'POST',
      body: JSON.stringify(goal),
    });
  }

  /**
   * Update goal status
   */
  async updateGoalStatus(
    manifoldId: string, 
    goalId: string, 
    status: GoalStatus
  ): Promise<Goal> {
    return this.request<Goal>(
      `/api/v1/manifolds/${manifoldId}/goals/${goalId}/status`,
      {
        method: 'PATCH',
        body: JSON.stringify({ status }),
      }
    );
  }

  /**
   * Validate all invariants in a manifold
   */
  async validateInvariants(manifoldId: string): Promise<InvariantValidationResult> {
    return this.request<InvariantValidationResult>(
      `/api/v1/manifolds/${manifoldId}/validate`
    );
  }

  /**
   * Get alignment score for current state
   */
  async getAlignmentScore(manifoldId: string): Promise<AlignmentScore> {
    return this.request<AlignmentScore>(
      `/api/v1/manifolds/${manifoldId}/alignment`
    );
  }

  /**
   * Subscribe to real-time updates
   */
  subscribeToUpdates(manifoldId: string, callback: (update: ManifoldUpdate) => void): () => void {
    // WebSocket implementation would go here
    const ws = new WebSocket(`${this.config.baseUrl.replace('http', 'ws')}/ws/manifolds/${manifoldId}`);
    
    ws.onmessage = (event) => {
      const update: ManifoldUpdate = JSON.parse(event.data);
      callback(update);
    };

    return () => ws.close();
  }

  private async request<T>(path: string, options: RequestInit = {}): Promise<T> {
    const url = `${this.config.baseUrl}${path}`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      ...(this.config.authToken && { Authorization: `Bearer ${this.config.authToken}` }),
      ...((options.headers as Record<string, string>) || {}),
    };

    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), this.config.timeout);

    try {
      const response = await fetch(url, {
        ...options,
        headers,
        signal: controller.signal,
      });

      clearTimeout(timeout);

      if (!response.ok) {
        const error = await response.text();
        throw new SentinelError(`HTTP ${response.status}: ${error}`, response.status);
      }

      return await response.json() as T;
    } catch (error) {
      clearTimeout(timeout);
      throw error;
    }
  }
}

export interface InvariantValidationResult {
  passed: boolean;
  violations: InvariantViolation[];
}

export interface InvariantViolation {
  invariantId: string;
  description: string;
  severity: 'warning' | 'error' | 'critical';
}

export interface AlignmentScore {
  score: number;
  confidence: number;
  violations: AlignmentViolation[];
}

export interface AlignmentViolation {
  id: string;
  description: string;
  severity: 'warning' | 'error' | 'critical';
}

export interface ManifoldUpdate {
  type: 'goal_added' | 'goal_updated' | 'goal_completed' | 'invariant_violated';
  timestamp: string;
  data: unknown;
}

export class SentinelError extends Error {
  constructor(message: string, public statusCode?: number) {
    super(message);
    this.name = 'SentinelError';
  }
}

// Re-export types
export { SentinelConfig as Config };
export default SentinelClient;
