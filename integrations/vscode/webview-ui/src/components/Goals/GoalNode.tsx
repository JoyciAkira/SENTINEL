import React from 'react';
import type { GoalNodeState } from '../../state/types';

const STATUS_ICONS: Record<string, string> = {
    Completed: '\u2705',
    InProgress: '\uD83D\uDD04',
    Validating: '\uD83D\uDD04',
    Ready: '\u23F3',
    Pending: '\u23F3',
    Blocked: '\u26D4',
    Failed: '\u274C',
    Deprecated: '\u23F8',
};

interface Props {
    goal: GoalNodeState;
}

export default function GoalNode({ goal }: Props) {
    const icon = STATUS_ICONS[goal.status] ?? '\u2753';

    return (
        <div style={{
            padding: '2px 0',
            fontSize: '12px',
            display: 'flex',
            gap: '6px',
            alignItems: 'flex-start',
        }}>
            <span>{icon}</span>
            <span style={{ flex: 1 }}>{goal.description}</span>
        </div>
    );
}
