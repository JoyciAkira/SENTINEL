import React, {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { useStore } from "../../state/store";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";

interface CriterionDraft {
  id: string;
  type:
    | "file_exists"
    | "directory_exists"
    | "command_succeeds"
    | "tests_passing"
    | "api_endpoint"
    | "performance";
  path?: string;
  command?: string;
  args?: string;
  expected_exit_code?: string;
  suite?: string;
  min_coverage?: string;
  url?: string;
  expected_status?: string;
  expected_body_contains?: string;
  metric?: string;
  threshold?: string;
  comparison?: string;
}

interface GoalDraft {
  id: string;
  description: string;
  scopeIn: string;
  scopeOut: string;
  deliverables: string;
  constraints: string;
  successCriteria: CriterionDraft[];
}

const CRITERION_TYPES = [
  { value: "file_exists", label: "File exists" },
  { value: "directory_exists", label: "Directory exists" },
  { value: "command_succeeds", label: "Command succeeds" },
  { value: "tests_passing", label: "Tests passing" },
  { value: "api_endpoint", label: "API endpoint" },
  { value: "performance", label: "Performance metric" },
];

const COMPARISONS = ["<", "<=", "==", ">=", ">"];

const buildCriterion = (type: CriterionDraft["type"]): CriterionDraft => ({
  id: crypto.randomUUID(),
  type,
  comparison: "<=",
});

const buildGoal = (description = ""): GoalDraft => ({
  id: crypto.randomUUID(),
  description,
  scopeIn: "",
  scopeOut: "",
  deliverables: "",
  constraints: "",
  successCriteria: [buildCriterion("file_exists")],
});

const splitLines = (value: string) =>
  value
    .split("\n")
    .map((line) => line.trim())
    .filter(Boolean);

const splitComma = (value: string) =>
  value
    .split(/[,\n]/)
    .map((item) => item.trim())
    .filter(Boolean);

export default function GoalBuilder() {
  const connected = useStore((s) => s.connected);
  const addMessage = useStore((s) => s.addMessage);
  const setGoals = useStore((s) => s.setGoals);
  const vscode = useVSCodeAPI();

  const [intent, setIntent] = useState("");
  const [constraints, setConstraints] = useState("");
  const [expectedOutcomes, setExpectedOutcomes] = useState("");
  const [targetPlatform, setTargetPlatform] = useState("");
  const [languages, setLanguages] = useState("");
  const [frameworks, setFrameworks] = useState("");
  const [goals, setLocalGoals] = useState<GoalDraft[]>([buildGoal()]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [suggesting, setSuggesting] = useState(false);

  const suggestIdRef = useRef<string | null>(null);
  const initIdRef = useRef<string | null>(null);

  const updateGoal = useCallback((id: string, patch: Partial<GoalDraft>) => {
    setLocalGoals((items) =>
      items.map((goal) => (goal.id === id ? { ...goal, ...patch } : goal)),
    );
  }, []);

  const updateCriterion = useCallback(
    (goalId: string, criterionId: string, patch: Partial<CriterionDraft>) => {
      setLocalGoals((items) =>
        items.map((goal) => {
          if (goal.id !== goalId) return goal;
          return {
            ...goal,
            successCriteria: goal.successCriteria.map((criterion) =>
              criterion.id === criterionId
                ? { ...criterion, ...patch }
                : criterion,
            ),
          };
        }),
      );
    },
    [],
  );

  const addGoal = () => setLocalGoals((items) => [...items, buildGoal()]);

  const removeGoal = (id: string) =>
    setLocalGoals((items) => items.filter((goal) => goal.id !== id));

  const addCriterion = (goalId: string) =>
    setLocalGoals((items) =>
      items.map((goal) =>
        goal.id === goalId
          ? {
              ...goal,
              successCriteria: [
                ...goal.successCriteria,
                buildCriterion("file_exists"),
              ],
            }
          : goal,
      ),
    );

  const removeCriterion = (goalId: string, criterionId: string) =>
    setLocalGoals((items) =>
      items.map((goal) =>
        goal.id === goalId
          ? {
              ...goal,
              successCriteria: goal.successCriteria.filter(
                (criterion) => criterion.id !== criterionId,
              ),
            }
          : goal,
      ),
    );

  const validate = useCallback(() => {
    if (!intent.trim()) return "Intent is required.";
    if (goals.length === 0) return "Add at least one goal.";

    for (const goal of goals) {
      if (!goal.description.trim()) return "Each goal needs a description.";
      if (goal.successCriteria.length === 0)
        return "Each goal needs success criteria.";

      for (const criterion of goal.successCriteria) {
        switch (criterion.type) {
          case "file_exists":
          case "directory_exists":
            if (!criterion.path?.trim())
              return "Provide a path for file/directory criteria.";
            break;
          case "command_succeeds":
            if (!criterion.command?.trim())
              return "Command criteria needs a command.";
            break;
          case "tests_passing":
            if (!criterion.suite?.trim())
              return "Tests criteria needs a suite name.";
            break;
          case "api_endpoint":
            if (!criterion.url?.trim()) return "API criteria needs a URL.";
            if (!criterion.expected_status?.trim())
              return "API criteria needs expected status.";
            break;
          case "performance":
            if (!criterion.metric?.trim())
              return "Performance criteria needs a metric.";
            if (!criterion.threshold?.trim())
              return "Performance criteria needs a threshold.";
            break;
        }
      }
    }

    return null;
  }, [intent, goals]);

  const canSubmit = useMemo(
    () => !validate() && connected && !busy,
    [validate, connected, busy],
  );

  const extractPayload = useCallback((result: any) => {
    if (!result) return null;
    if (typeof result === "string") return result;
    if (typeof result.text === "string") return result.text;
    if (result.content?.[0]?.text) return result.content[0].text;
    return result;
  }, []);

  useEffect(() => {
    const handler = (event: MessageEvent) => {
      const msg = event.data;
      if (msg?.type !== "mcpResponse") return;

      if (msg.id && msg.id === suggestIdRef.current) {
        setSuggesting(false);
        suggestIdRef.current = null;
        if (msg.error) {
          setError(String(msg.error));
          return;
        }
        const payload = extractPayload(msg.result);
        const suggestions = Array.isArray(payload)
          ? payload
          : Array.isArray(payload?.goals)
            ? payload.goals
            : [];
        if (suggestions.length === 0 && typeof payload === "string") {
          try {
            const parsed = JSON.parse(payload);
            if (Array.isArray(parsed)) suggestions.push(...parsed);
            if (Array.isArray(parsed?.goals)) suggestions.push(...parsed.goals);
          } catch {
            // ignore parse failures
          }
        }
        if (suggestions.length === 0) {
          setError("No suggestions returned.");
          return;
        }
        setLocalGoals(
          suggestions.map((item: any) =>
            buildGoal(typeof item === "string" ? item : item.description || ""),
          ),
        );
        setError(null);
      }

      if (msg.id && msg.id === initIdRef.current) {
        setBusy(false);
        initIdRef.current = null;
        if (msg.error) {
          setError(String(msg.error));
          return;
        }
        const payload = extractPayload(msg.result);
        if (!payload) {
          setError("Initialization failed.");
          return;
        }
        addMessage({
          id: crypto.randomUUID(),
          role: "assistant",
          content:
            "\u2705 Goal Manifold created. Switch to Atomic Forge to inspect the graph.",
          timestamp: Date.now(),
        });
        vscode.postMessage({ type: "refreshGoals" });
        setError(null);
      }
    };

    window.addEventListener("message", handler);
    return () => window.removeEventListener("message", handler);
  }, [addMessage, vscode, extractPayload]);

  const handleSuggest = () => {
    const validation = validate();
    if (!intent.trim()) {
      setError("Add a clear intent before generating suggestions.");
      return;
    }
    if (validation && validation !== "Each goal needs a description.") {
      setError(validation);
      return;
    }

    setSuggesting(true);
    const id = crypto.randomUUID();
    suggestIdRef.current = id;

    vscode.postMessage({
      command: "mcpRequest",
      method: "tools/call",
      params: {
        name: "suggest_goals",
        arguments: {
          description: intent.trim(),
          constraints: splitLines(constraints),
          expected_outcomes: splitLines(expectedOutcomes),
          target_platform: targetPlatform.trim() || undefined,
          languages: splitComma(languages),
          frameworks: splitComma(frameworks),
        },
      },
      id,
    });
  };

  const handleInit = () => {
    const validation = validate();
    if (validation) {
      setError(validation);
      return;
    }

    setBusy(true);
    const id = crypto.randomUUID();
    initIdRef.current = id;

    const payload = {
      description: intent.trim(),
      constraints: splitLines(constraints),
      expected_outcomes: splitLines(expectedOutcomes),
      target_platform: targetPlatform.trim() || undefined,
      languages: splitComma(languages),
      frameworks: splitComma(frameworks),
      goals: goals.map((goal) => ({
        description: goal.description.trim(),
        scope_in: splitLines(goal.scopeIn),
        scope_out: splitLines(goal.scopeOut),
        deliverables: splitLines(goal.deliverables),
        constraints: splitLines(goal.constraints),
        success_criteria: goal.successCriteria.map((criterion) => ({
          type: criterion.type,
          path: criterion.path?.trim(),
          command: criterion.command?.trim(),
          args: criterion.args
            ? criterion.args.split(/\s+/).filter(Boolean)
            : undefined,
          expected_exit_code: criterion.expected_exit_code
            ? Number(criterion.expected_exit_code)
            : undefined,
          suite: criterion.suite?.trim(),
          min_coverage: criterion.min_coverage
            ? Number(criterion.min_coverage)
            : undefined,
          url: criterion.url?.trim(),
          expected_status: criterion.expected_status
            ? Number(criterion.expected_status)
            : undefined,
          expected_body_contains: criterion.expected_body_contains?.trim(),
          metric: criterion.metric?.trim(),
          threshold: criterion.threshold
            ? Number(criterion.threshold)
            : undefined,
          comparison: criterion.comparison,
        })),
      })),
    };

    vscode.postMessage({
      command: "mcpRequest",
      method: "tools/call",
      params: {
        name: "init_project",
        arguments: payload,
      },
      id,
    });
  };

  useEffect(() => {
    if (connected) return;
    setError("Connect Sentinel to create goals.");
  }, [connected]);

  return (
    <div className="goal-builder">
      <div className="goal-builder__header">
        <div>
          <div className="section-title">Start Project (Strict Mode)</div>
          <div className="chat-subtitle">
            Define goals explicitly. Sentinel will only use approved criteria.
          </div>
        </div>
        <div className="goal-builder__actions">
          <button
            type="button"
            className="btn btn--subtle"
            disabled={!connected || suggesting}
            onClick={handleSuggest}
          >
            {suggesting ? "Suggesting..." : "Suggest Goals"}
          </button>
          <button
            type="button"
            className="btn"
            disabled={!canSubmit}
            onClick={handleInit}
          >
            {busy ? "Creating..." : "Create Goal Manifold"}
          </button>
        </div>
      </div>

      {error && <div className="goal-builder__error">{error}</div>}

      <div className="goal-builder__section">
        <div className="goal-builder__label">Intent</div>
        <textarea
          className="goal-builder__textarea"
          value={intent}
          onChange={(e) => setIntent(e.target.value)}
          placeholder="Describe the project in one sentence"
        />
      </div>

      <div className="goal-builder__grid">
        <div className="goal-builder__section">
          <div className="goal-builder__label">Constraints (one per line)</div>
          <textarea
            className="goal-builder__textarea"
            value={constraints}
            onChange={(e) => setConstraints(e.target.value)}
            placeholder="Use Rust\nNo external DB\nOffline-first"
          />
        </div>
        <div className="goal-builder__section">
          <div className="goal-builder__label">
            Expected Outcomes (one per line)
          </div>
          <textarea
            className="goal-builder__textarea"
            value={expectedOutcomes}
            onChange={(e) => setExpectedOutcomes(e.target.value)}
            placeholder="Auth API live\nCoverage >= 80%"
          />
        </div>
      </div>

      <div className="goal-builder__grid">
        <div className="goal-builder__section">
          <div className="goal-builder__label">Target Platform</div>
          <input
            className="goal-builder__input"
            value={targetPlatform}
            onChange={(e) => setTargetPlatform(e.target.value)}
            placeholder="macOS / Linux / Cloud"
          />
        </div>
        <div className="goal-builder__section">
          <div className="goal-builder__label">Languages (comma separated)</div>
          <input
            className="goal-builder__input"
            value={languages}
            onChange={(e) => setLanguages(e.target.value)}
            placeholder="Rust, TypeScript"
          />
        </div>
        <div className="goal-builder__section">
          <div className="goal-builder__label">
            Frameworks (comma separated)
          </div>
          <input
            className="goal-builder__input"
            value={frameworks}
            onChange={(e) => setFrameworks(e.target.value)}
            placeholder="Axum, React"
          />
        </div>
      </div>

      <div className="goal-builder__goals">
        <div className="goal-builder__row">
          <div className="section-title">Goals</div>
          <button type="button" className="btn btn--ghost" onClick={addGoal}>
            Add Goal
          </button>
        </div>

        {goals.map((goal, index) => (
          <div key={goal.id} className="goal-card">
            <div className="goal-card__header">
              <div className="section-title">Goal {index + 1}</div>
              <button
                type="button"
                className="btn btn--subtle"
                onClick={() => removeGoal(goal.id)}
                disabled={goals.length === 1}
              >
                Remove
              </button>
            </div>

            <div className="goal-builder__section">
              <div className="goal-builder__label">Goal description</div>
              <input
                className="goal-builder__input"
                value={goal.description}
                onChange={(e) =>
                  updateGoal(goal.id, { description: e.target.value })
                }
                placeholder="Implement JWT login endpoint"
              />
            </div>

            <div className="goal-builder__grid">
              <div className="goal-builder__section">
                <div className="goal-builder__label">
                  Scope In (one per line)
                </div>
                <textarea
                  className="goal-builder__textarea"
                  value={goal.scopeIn}
                  onChange={(e) =>
                    updateGoal(goal.id, { scopeIn: e.target.value })
                  }
                  placeholder="Login flow\nToken issuance"
                />
              </div>
              <div className="goal-builder__section">
                <div className="goal-builder__label">
                  Scope Out (one per line)
                </div>
                <textarea
                  className="goal-builder__textarea"
                  value={goal.scopeOut}
                  onChange={(e) =>
                    updateGoal(goal.id, { scopeOut: e.target.value })
                  }
                  placeholder="Refresh tokens\nUser profile"
                />
              </div>
            </div>

            <div className="goal-builder__grid">
              <div className="goal-builder__section">
                <div className="goal-builder__label">
                  Deliverables (one per line)
                </div>
                <textarea
                  className="goal-builder__textarea"
                  value={goal.deliverables}
                  onChange={(e) =>
                    updateGoal(goal.id, { deliverables: e.target.value })
                  }
                  placeholder="src/auth/login.ts\nTests for auth"
                />
              </div>
              <div className="goal-builder__section">
                <div className="goal-builder__label">
                  Goal Constraints (one per line)
                </div>
                <textarea
                  className="goal-builder__textarea"
                  value={goal.constraints}
                  onChange={(e) =>
                    updateGoal(goal.id, { constraints: e.target.value })
                  }
                  placeholder="No external auth provider"
                />
              </div>
            </div>

            <div className="goal-builder__criteria">
              <div className="goal-builder__row">
                <div className="section-title">Success Criteria</div>
                <button
                  type="button"
                  className="btn btn--ghost"
                  onClick={() => addCriterion(goal.id)}
                >
                  Add Criterion
                </button>
              </div>

              {goal.successCriteria.map((criterion) => (
                <div key={criterion.id} className="criterion-row">
                  <select
                    className="goal-builder__select"
                    value={criterion.type}
                    onChange={(e) =>
                      updateCriterion(goal.id, criterion.id, {
                        type: e.target.value as CriterionDraft["type"],
                      })
                    }
                  >
                    {CRITERION_TYPES.map((opt) => (
                      <option key={opt.value} value={opt.value}>
                        {opt.label}
                      </option>
                    ))}
                  </select>

                  {criterion.type === "file_exists" ||
                  criterion.type === "directory_exists" ? (
                    <input
                      className="goal-builder__input"
                      value={criterion.path ?? ""}
                      onChange={(e) =>
                        updateCriterion(goal.id, criterion.id, {
                          path: e.target.value,
                        })
                      }
                      placeholder="path/to/file"
                    />
                  ) : null}

                  {criterion.type === "command_succeeds" ? (
                    <>
                      <input
                        className="goal-builder__input"
                        value={criterion.command ?? ""}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            command: e.target.value,
                          })
                        }
                        placeholder="command"
                      />
                      <input
                        className="goal-builder__input"
                        value={criterion.args ?? ""}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            args: e.target.value,
                          })
                        }
                        placeholder="args"
                      />
                      <input
                        className="goal-builder__input"
                        value={criterion.expected_exit_code ?? "0"}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            expected_exit_code: e.target.value,
                          })
                        }
                        placeholder="exit code"
                      />
                    </>
                  ) : null}

                  {criterion.type === "tests_passing" ? (
                    <>
                      <input
                        className="goal-builder__input"
                        value={criterion.suite ?? ""}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            suite: e.target.value,
                          })
                        }
                        placeholder="suite name"
                      />
                      <input
                        className="goal-builder__input"
                        value={criterion.min_coverage ?? "0.8"}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            min_coverage: e.target.value,
                          })
                        }
                        placeholder="min coverage"
                      />
                    </>
                  ) : null}

                  {criterion.type === "api_endpoint" ? (
                    <>
                      <input
                        className="goal-builder__input"
                        value={criterion.url ?? ""}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            url: e.target.value,
                          })
                        }
                        placeholder="/api/endpoint"
                      />
                      <input
                        className="goal-builder__input"
                        value={criterion.expected_status ?? "200"}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            expected_status: e.target.value,
                          })
                        }
                        placeholder="status"
                      />
                      <input
                        className="goal-builder__input"
                        value={criterion.expected_body_contains ?? ""}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            expected_body_contains: e.target.value,
                          })
                        }
                        placeholder="body contains (optional)"
                      />
                    </>
                  ) : null}

                  {criterion.type === "performance" ? (
                    <>
                      <input
                        className="goal-builder__input"
                        value={criterion.metric ?? ""}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            metric: e.target.value,
                          })
                        }
                        placeholder="metric"
                      />
                      <input
                        className="goal-builder__input"
                        value={criterion.threshold ?? ""}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            threshold: e.target.value,
                          })
                        }
                        placeholder="threshold"
                      />
                      <select
                        className="goal-builder__select"
                        value={criterion.comparison ?? "<="}
                        onChange={(e) =>
                          updateCriterion(goal.id, criterion.id, {
                            comparison: e.target.value,
                          })
                        }
                      >
                        {COMPARISONS.map((cmp) => (
                          <option key={cmp} value={cmp}>
                            {cmp}
                          </option>
                        ))}
                      </select>
                    </>
                  ) : null}

                  <button
                    type="button"
                    className="btn btn--subtle"
                    onClick={() => removeCriterion(goal.id, criterion.id)}
                    disabled={goal.successCriteria.length === 1}
                  >
                    Remove
                  </button>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
