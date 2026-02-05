import React, { useState, useEffect, useRef, useMemo } from 'react';
import { Goal } from '@sentinel/sdk';

interface GraphNode extends Goal {
    x: number;
    y: number;
}

interface GoalGraphProps {
    goals: Goal[];
    onNodeSelect: (goal: Goal) => void;
}

const GoalGraph: React.FC<GoalGraphProps> = ({ goals, onNodeSelect }) => {
    const [nodes, setNodes] = useState<GraphNode[]>([]);
    const [draggingId, setDraggingId] = useState<string | null>(null);
    const [offset, setOffset] = useState({ x: 0, y: 0 });
    const [viewOffset, setViewOffset] = useState({ x: 0, y: 0 });
    const [isPanning, setIsPanning] = useState(false);
    const [panStart, setPanStart] = useState({ x: 0, y: 0 });
    
    const containerRef = useRef<HTMLDivElement>(null);

    // Initial Layout: Hierarchical-ish layout
    useEffect(() => {
        if (goals.length > 0) {
            setNodes(prev => {
                return goals.map((g, i) => {
                    const existing = prev.find(n => n.id === g.id);
                    if (existing) return { ...g, x: existing.x, y: existing.y };
                    
                    // Logic to spread nodes if new
                    return {
                        ...g,
                        x: 150 + (i % 2) * 350 + Math.random() * 20,
                        y: 100 + Math.floor(i / 2) * 180 + Math.random() * 20
                    };
                });
            });
        }
    }, [goals]);

    const handleMouseDown = (e: React.MouseEvent, id: string, x: number, y: number) => {
        e.stopPropagation();
        setDraggingId(id);
        setOffset({
            x: e.clientX - x,
            y: e.clientY - y
        });
    };

    const handleBackgroundMouseDown = (e: React.MouseEvent) => {
        if (e.button === 0 || e.button === 1) { // Left or middle click for pan
            setIsPanning(true);
            setPanStart({ x: e.clientX - viewOffset.x, y: e.clientY - viewOffset.y });
        }
    };

    const handleMouseMove = (e: React.MouseEvent) => {
        if (draggingId) {
            setNodes(prev => prev.map(n => {
                if (n.id === draggingId) {
                    return { ...n, x: e.clientX - offset.x, y: e.clientY - offset.y };
                }
                return n;
            }));
        } else if (isPanning) {
            setViewOffset({
                x: e.clientX - panStart.x,
                y: e.clientY - panStart.y
            });
        }
    };

    const handleMouseUp = () => {
        setDraggingId(null);
        setIsPanning(false);
    };

    const getStatusColor = (status: string) => {
        switch (status) {
            case 'Completed': return 'var(--accent)';
            case 'InProgress': return '#3b82f6';
            case 'Blocked': return 'var(--danger)';
            case 'Ready': return 'var(--warning)';
            default: return 'var(--text-subtle)';
        }
    };

    return (
        <div 
            ref={containerRef}
            className="forge-canvas"
            style={{ 
                width: '100%', 
                height: '100%', 
                position: 'relative', 
                cursor: isPanning ? 'grabbing' : 'crosshair',
                backgroundImage: `radial-gradient(circle, var(--border) 1px, transparent 1px)`,
                backgroundSize: '24px 24px',
                backgroundPosition: `${viewOffset.x}px ${viewOffset.y}px`,
                backgroundColor: 'var(--bg-surface)',
                borderStyle: 'solid'
            }}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseDown={handleBackgroundMouseDown}
            onMouseLeave={handleMouseUp}
        >
            <svg 
                width="100%" 
                height="100%" 
                style={{ position: 'absolute', top: 0, left: 0, pointerEvents: 'none', overflow: 'visible' }}
            >
                <defs>
                    <filter id="glow" x="-20%" y="-20%" width="140%" height="140%">
                        <feGaussianBlur stdDeviation="3" result="blur" />
                        <feComposite in="SourceGraphic" in2="blur" operator="over" />
                    </filter>
                    <marker id="arrowhead" markerWidth="10" markerHeight="7" refX="9" refY="3.5" orient="auto">
                        <polygon points="0 0, 10 3.5, 0 7" fill="var(--text-subtle)" />
                    </marker>
                </defs>
                
                <g transform={`translate(${viewOffset.x}, ${viewOffset.y})`}>
                    {nodes.map(node => (
                        (node.dependencies || []).map(depId => {
                            const source = nodes.find(n => n.id === depId);
                            if (!source) return null;
                            
                            const x1 = source.x + 240;
                            const y1 = source.y + 40;
                            const x2 = node.x;
                            const y2 = node.y + 40;
                            
                            // Curved bezier path
                            const dx = Math.abs(x2 - x1) * 0.5;
                            const pathData = `M ${x1} ${y1} C ${x1 + dx} ${y1}, ${x2 - dx} ${y2}, ${x2} ${y2}`;

                            return (
                                <path 
                                    key={`${source.id}-${node.id}`}
                                    d={pathData}
                                    stroke="var(--border)"
                                    strokeWidth="2"
                                    fill="none"
                                    markerEnd="url(#arrowhead)"
                                    style={{ transition: 'stroke 0.3s' }}
                                />
                            );
                        })
                    ))}
                </g>
            </svg>

            <div style={{ transform: `translate(${viewOffset.x}px, ${viewOffset.y}px)`, position: 'absolute', top: 0, left: 0 }}>
                {nodes.map(node => (
                    <div
                        key={node.id}
                        className="card"
                        style={{
                            position: 'absolute',
                            left: node.x,
                            top: node.y,
                            width: '280px',
                            padding: '12px 16px',
                            cursor: draggingId === node.id ? 'grabbing' : 'grab',
                            borderColor: draggingId === node.id ? 'var(--accent)' : 'var(--border)',
                            boxShadow: draggingId === node.id ? '0 20px 40px rgba(0,0,0,0.2)' : 'var(--shadow)',
                            zIndex: draggingId === node.id ? 100 : 10,
                            transition: draggingId ? 'none' : 'all 0.2s ease',
                            background: 'var(--bg-surface-2)',
                            backdropFilter: 'blur(8px)',
                            borderLeft: `4px solid ${getStatusColor(node.status)}`
                        }}
                        onMouseDown={(e) => handleMouseDown(e, node.id, node.x, node.y)}
                        onClick={() => onNodeSelect(node)}
                    >
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '8px' }}>
                            <span className="mono" style={{ fontSize: '10px' }}>{node.id.slice(0, 8)}</span>
                            <span className="chip" style={{ 
                                fontSize: '9px', 
                                padding: '2px 8px', 
                                background: 'transparent',
                                border: `1px solid ${getStatusColor(node.status)}`,
                                color: getStatusColor(node.status)
                            }}>
                                {node.status}
                            </span>
                        </div>
                        <div style={{ 
                            fontWeight: 600, 
                            fontSize: '13px', 
                            color: 'var(--text-strong)',
                            lineHeight: 1.4
                        }}>
                            {node.description}
                        </div>
                        
                        <div style={{ marginTop: '12px', display: 'flex', gap: '12px', alignItems: 'center' }}>
                           <div className="meter" style={{ flex: 1, height: '4px' }}>
                              <span style={{ 
                                  width: node.status === 'Completed' ? '100%' : node.status === 'InProgress' ? '40%' : '0%',
                                  background: getStatusColor(node.status)
                              }} />
                           </div>
                           <span className="mono" style={{ fontSize: '10px' }}>
                             {node.status === 'Completed' ? '1.0' : '0.4'}
                           </span>
                        </div>
                    </div>
                ))}
            </div>

            {/* Floating Controls */}
            <div style={{ position: 'absolute', bottom: 20, right: 20, display: 'flex', gap: '8px' }}>
                <button className="nav-btn" style={{ padding: '8px 12px' }} onClick={() => setViewOffset({x:0, y:0})}>Recenter</button>
                <div className="status-pill">Atomic Forge v2.0</div>
            </div>
        </div>
    );
};

export default GoalGraph;