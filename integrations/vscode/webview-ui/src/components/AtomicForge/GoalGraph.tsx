import React, { useCallback, useEffect, useMemo } from 'react';
import ReactFlow, { 
    Node, 
    Edge, 
    Background, 
    Controls, 
    useNodesState, 
    useEdgesState, 
    MarkerType,
    Handle,
    Position
} from 'reactflow';
import 'reactflow/dist/style.css';
import { Goal } from '@sentinel/sdk';
import { Badge } from '../ui/badge';
import { cn } from '@/lib/utils';
import { Target, CheckCircle2, Clock, AlertTriangle } from 'lucide-react';

interface GoalGraphProps {
    goals: Goal[];
    onNodeSelect: (goal: Goal) => void;
}

const GoalNode = ({ data }: { data: Goal }) => {
  const statusColor = useMemo(() => {
    switch(data.status?.toLowerCase()) {
        case 'completed': return 'border-green-500 bg-green-500/10 text-green-500';
        case 'inprogress': return 'border-blue-500 bg-blue-500/10 text-blue-500';
        case 'blocked': return 'border-red-500 bg-red-500/10 text-red-500';
        default: return 'border-gray-500 bg-gray-500/10 text-gray-500';
    }
  }, [data.status]);

  const StatusIcon = useMemo(() => {
      switch(data.status?.toLowerCase()) {
          case 'completed': return CheckCircle2;
          case 'inprogress': return Clock;
          case 'blocked': return AlertTriangle;
          default: return Target;
      }
  }, [data.status]);

  return (
    <div className={cn(
        "px-4 py-3 rounded-xl border-2 shadow-lg min-w-[200px] bg-card transition-all hover:scale-105",
        statusColor
    )}>
      <Handle type="target" position={Position.Top} className="!bg-muted-foreground !size-3" />
      
      <div className="flex items-center justify-between mb-2">
         <span className="font-mono text-[10px] opacity-60 uppercase tracking-widest">{data.id.slice(0, 6)}</span>
         <StatusIcon className="size-4" />
      </div>
      
      <div className="text-xs font-bold leading-snug mb-2 text-foreground">
        {data.description}
      </div>

      <div className="flex items-center gap-2">
         <Badge variant="outline" className="text-[10px] h-5 px-1.5 uppercase">
            {data.status || 'Pending'}
         </Badge>
         {data.value_to_root && (
             <span className="text-[9px] font-mono text-muted-foreground">Val: {data.value_to_root.toFixed(2)}</span>
         )}
      </div>

      <Handle type="source" position={Position.Bottom} className="!bg-muted-foreground !size-3" />
    </div>
  );
};

const nodeTypes = {
  goal: GoalNode,
};

const GoalGraph: React.FC<GoalGraphProps> = ({ goals, onNodeSelect }) => {
    const [nodes, setNodes, onNodesChange] = useNodesState([]);
    const [edges, setEdges, onEdgesChange] = useEdgesState([]);

    useEffect(() => {
        if (!goals.length) return;

        // Simple hierarchical layout calculation
        // In a real world-class app, use dagre for auto-layout
        const newNodes: Node[] = goals.map((g, i) => ({
            id: g.id,
            type: 'goal',
            data: g,
            position: { 
                x: 250 * (i % 3), 
                y: 150 * Math.floor(i / 3) 
            },
        }));

        const newEdges: Edge[] = [];
        goals.forEach(g => {
            if (g.dependencies) {
                g.dependencies.forEach(depId => {
                    newEdges.push({
                        id: `${depId}-${g.id}`,
                        source: depId,
                        target: g.id,
                        animated: g.status === 'InProgress',
                        style: { stroke: '#64748b', strokeWidth: 2 },
                        markerEnd: { type: MarkerType.ArrowClosed, color: '#64748b' },
                    });
                });
            }
        });

        setNodes(newNodes);
        setEdges(newEdges);
    }, [goals, setNodes, setEdges]);

    const onNodeClick = useCallback((_: React.MouseEvent, node: Node) => {
        onNodeSelect(node.data);
    }, [onNodeSelect]);

    return (
        <div className="w-full h-full bg-background/50 rounded-xl overflow-hidden border border-border/50">
            <ReactFlow
                nodes={nodes}
                edges={edges}
                onNodesChange={onNodesChange}
                onEdgesChange={onEdgesChange}
                onNodeClick={onNodeClick}
                nodeTypes={nodeTypes}
                fitView
                fitViewOptions={{ padding: 0.2 }}
                minZoom={0.5}
                maxZoom={2}
                proOptions={{ hideAttribution: true }}
            >
                <Background color="#333" gap={20} size={1} className="opacity-10" />
                <Controls className="!bg-card !border !border-border !shadow-sm !rounded-lg overflow-hidden [&>button]:!border-border/50 [&>button]:!bg-card hover:[&>button]:!bg-accent" />
            </ReactFlow>
        </div>
    );
};

export default GoalGraph;
