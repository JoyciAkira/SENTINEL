import React, { useState, useEffect, useRef } from 'react';
import { Goal } from '@sentinel/sdk'; // Importing interfaces from SDK

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
    const svgRef = useRef<SVGSVGElement>(null);

    // Initial Layout Logic
    useEffect(() => {
        if (nodes.length === 0 && goals.length > 0) {
            const initialNodes = goals.map((g, i) => ({
                ...g,
                // Simple grid layout for initialization
                x: 100 + (i % 3) * 300,
                y: 100 + Math.floor(i / 3) * 200
            }));
            setNodes(initialNodes);
        }
    }, [goals]);

    // Update nodes when goals change but preserve positions
    useEffect(() => {
        setNodes(prev => {
            return goals.map(g => {
                const existing = prev.find(n => n.id === g.id);
                return existing ? { ...g, x: existing.x, y: existing.y } : { ...g, x: 100, y: 100 };
            });
        });
    }, [goals]);

    const handleMouseDown = (e: React.MouseEvent, id: string, x: number, y: number) => {
        setDraggingId(id);
        setOffset({
            x: e.clientX - x,
            y: e.clientY - y
        });
    };

    const handleMouseMove = (e: React.MouseEvent) => {
        if (!draggingId) return;
        setNodes(prev => prev.map(n => {
            if (n.id === draggingId) {
                return { ...n, x: e.clientX - offset.x, y: e.clientY - offset.y };
            }
            return n;
        }));
    };

    const handleMouseUp = () => {
        setDraggingId(null);
    };

    return (
        <div 
            style={{ width: '100%', height: '100%', overflow: 'hidden', position: 'relative', background: '#050505' }}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
        >
            <svg 
                ref={svgRef}
                width="100%" 
                height="100%" 
                style={{ position: 'absolute', top: 0, left: 0, pointerEvents: 'none' }}
            >
                <defs>
                    <marker id="arrow" markerWidth="10" markerHeight="10" refX="20" refY="3" orient="auto" markerUnits="strokeWidth">
                        <path d="M0,0 L0,6 L9,3 z" fill="#858585" />
                    </marker>
                </defs>
                {/* Edges */}
                {nodes.map(node => (
                    node.dependencies.map(depId => {
                        const source = nodes.find(n => n.id === depId);
                        if (!source) return null;
                        
                        const x1 = source.x + 240;
                        const y1 = source.y + 60;
                        const x2 = node.x;
                        const y2 = node.y + 60;
                        const c1x = x1 + (x2 - x1) / 2;
                        const c2x = x2 - (x2 - x1) / 2;

                        return (
                            <path 
                                key={`${source.id}-${node.id}`}
                                d={`M ${x1} ${y1} C ${c1x} ${y1}, ${c2x} ${y2}, ${x2} ${y2}`}
                                stroke="#444"
                                strokeWidth="2"
                                fill="none"
                                markerEnd="url(#arrow)"
                            />
                        );
                    })
                ))}
            </svg>

            {/* Nodes */}
            {nodes.map(node => (
                <div
                    key={node.id}
                    style={{
                        position: 'absolute',
                        left: node.x,
                        top: node.y,
                        width: '240px',
                        background: '#111',
                        border: `1px solid ${node.status === 'Completed' ? '#10b981' : '#333'}`,
                        borderRadius: '12px',
                        padding: '16px',
                        boxShadow: '0 10px 30px rgba(0,0,0,0.5)',
                        cursor: 'grab',
                        userSelect: 'none',
                        zIndex: 10
                    }}
                    onMouseDown={(e) => handleMouseDown(e, node.id, node.x, node.y)}
                    onClick={() => onNodeSelect(node)}
                >
                    <div style={{display:'flex', justifyContent:'space-between', marginBottom:'8px', fontSize:'11px', color:'#666'}}>
                        <span style={{fontFamily:'monospace'}}>{node.id.slice(0,8)}</span>
                        <span style={{
                            width:'8px', height:'8px', borderRadius:'50%', 
                            background: node.status === 'Completed' ? '#10b981' : node.status === 'InProgress' ? '#3b82f6' : '#555'
                        }}></span>
                    </div>
                    <div style={{fontWeight:600, fontSize:'14px', color:'#eee'}}>{node.description}</div>
                </div>
            ))}
        </div>
    );
};

export default GoalGraph;