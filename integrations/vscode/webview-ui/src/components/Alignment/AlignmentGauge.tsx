import React from 'react';
import { useStore } from '../../state/store';
import { colors } from '../../utils/theme';

export default function AlignmentGauge() {
    const alignment = useStore((s) => s.alignment);

    if (!alignment) {
        return (
            <div style={containerStyle}>
                <span style={{ color: colors.descriptionFg, fontSize: '12px' }}>
                    Alignment: --
                </span>
            </div>
        );
    }

    const score = alignment.score;
    const trendArrow = alignment.trend > 0 ? '\u25B2' : alignment.trend < 0 ? '\u25BC' : '';
    const trendText = alignment.trend !== 0 ? ` ${trendArrow} ${Math.abs(alignment.trend).toFixed(1)}` : '';

    const barColor = score >= 75 ? '#4caf50' : score >= 40 ? '#ff9800' : '#f44336';
    const pct = Math.min(100, Math.max(0, score));

    return (
        <div style={containerStyle}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
                <span style={{ fontSize: '12px', fontWeight: 600 }}>
                    Alignment: {score.toFixed(1)}%{trendText}
                </span>
                <span style={{ fontSize: '11px', color: colors.descriptionFg }}>
                    {alignment.status}
                </span>
            </div>
            <div style={{
                width: '100%',
                height: '6px',
                backgroundColor: 'var(--vscode-progressBar-background, #333)',
                borderRadius: '3px',
                overflow: 'hidden',
            }}>
                <div style={{
                    width: `${pct}%`,
                    height: '100%',
                    backgroundColor: barColor,
                    borderRadius: '3px',
                    transition: 'width 0.3s ease',
                }} />
            </div>
        </div>
    );
}

const containerStyle: React.CSSProperties = {
    padding: '8px 12px',
    borderBottom: `1px solid var(--vscode-panel-border)`,
    flexShrink: 0,
};
