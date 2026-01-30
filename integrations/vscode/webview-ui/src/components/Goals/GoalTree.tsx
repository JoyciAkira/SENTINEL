import React from 'react';
import { useStore } from '../../state/store';
import GoalNode from './GoalNode';
import GoalProgress from './GoalProgress';

export default function GoalTree() {
    const goals = useStore((s) => s.goals);
    const collapsed = useStore((s) => s.goalsCollapsed);
    const toggle = useStore((s) => s.toggleGoalsCollapsed);

    if (goals.length === 0) return null;

    const completed = goals.filter((g) => g.status === 'Completed').length;
    const total = goals.length;

    return (
        <div style={{
            borderBottom: '1px solid var(--vscode-panel-border)',
            flexShrink: 0,
        }}>
            <div
                onClick={toggle}
                style={{
                    padding: '6px 12px',
                    cursor: 'pointer',
                    display: 'flex',
                    alignItems: 'center',
                    gap: '6px',
                    fontSize: '12px',
                    fontWeight: 600,
                    userSelect: 'none',
                }}
            >
                <span>{collapsed ? '\u25B6' : '\u25BC'}</span>
                <span>Goal Manifold</span>
                <GoalProgress completed={completed} total={total} />
            </div>
            {!collapsed && (
                <div style={{ padding: '0 12px 8px 24px' }}>
                    {goals.map((g) => (
                        <GoalNode key={g.id} goal={g} />
                    ))}
                </div>
            )}
        </div>
    );
}
