import React from 'react';

interface Props {
    completed: number;
    total: number;
}

export default function GoalProgress({ completed, total }: Props) {
    if (total === 0) return null;
    const pct = Math.round((completed / total) * 100);

    return (
        <span style={{ fontSize: '11px', color: 'var(--vscode-descriptionForeground)' }}>
            ({completed}/{total} done)
        </span>
    );
}
