// CommunicationGraph.tsx - Visual debugging of agent communication
// Shows real-time message flow between agents with D3.js/ReactFlow

import React, { useEffect, useState, useCallback, useMemo } from 'react';
import ReactFlow, {
  Node,
  Edge,
  addEdge,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  MarkerType,
  Position,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { useVSCodeAPI } from '../../hooks/useVSCodeAPI';

interface CommunicationMessage {
  id: string;
  type: string;
  from: string;
  to: string[];
  timestamp: number;
  payload: any;
}

interface AgentNode {
  id: string;
  name: string;
  type: string;
  status: 'idle' | 'working' | 'completed' | 'failed';
  x: number;
  y: number;
}

interface CommunicationGraphProps {
  height?: number;
}

const nodeColors: Record<string, string> = {
  AuthArchitect: '#FF6B6B',
  SecurityAuditor: '#4ECDC4',
  APICoder: '#45B7D1',
  FrontendCoder: '#96CEB4',
  DatabaseArchitect: '#FFEAA7',
  TestWriter: '#DDA0DD',
  DevOpsEngineer: '#98D8C8',
  Manager: '#F7DC6F',
  default: '#BDC3C7',
};

const CustomNode = ({ data }: { data: any }) => {
  const color = nodeColors[data.agentType] || nodeColors.default;
  
  return (
    <div
      style={{
        padding: '10px',
        borderRadius: '8px',
        background: color,
        color: '#fff',
        border: `2px solid ${data.status === 'working' ? '#fff' : 'transparent'}`,
        boxShadow: data.status === 'working' ? '0 0 10px rgba(255,255,255,0.5)' : 'none',
        minWidth: '120px',
        textAlign: 'center',
      }}
    >
      <div style={{ fontWeight: 'bold', fontSize: '12px' }}>{data.name}</div>
      <div style={{ fontSize: '10px', opacity: 0.9 }}>{data.agentType}</div>
      <div style={{ 
        fontSize: '9px', 
        marginTop: '4px',
        padding: '2px 6px',
        background: 'rgba(0,0,0,0.2)',
        borderRadius: '4px',
        display: 'inline-block'
      }}>
        {data.status}
      </div>
    </div>
  );
};

const nodeTypes = {
  custom: CustomNode,
};

export const CommunicationGraph: React.FC<CommunicationGraphProps> = ({ height = 500 }) => {
  const vscode = useVSCodeAPI();
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedMessage, setSelectedMessage] = useState<CommunicationMessage | null>(null);
  const [messageHistory, setMessageHistory] = useState<CommunicationMessage[]>([]);
  const [isPaused, setIsPaused] = useState(false);
  const [showLabels, setShowLabels] = useState(true);

  // Listen for communication messages from VSCode
  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      const message = event.data;
      
      if (message.type === 'communicationUpdate') {
        const commMsg: CommunicationMessage = message.payload;
        
        if (!isPaused) {
          setMessageHistory(prev => [...prev.slice(-50), commMsg]);
          updateGraphWithMessage(commMsg);
        }
      } else if (message.type === 'agentPositions') {
        updateAgentPositions(message.payload);
      }
    };

    window.addEventListener('message', handleMessage);
    vscode.postMessage({ type: 'getCommunicationGraph' });

    return () => window.removeEventListener('message', handleMessage);
  }, [isPaused, vscode]);

  // Update graph with new message
  const updateGraphWithMessage = useCallback((msg: CommunicationMessage) => {
    // Add edges for each recipient
    const newEdges: Edge[] = msg.to.map((recipientId, idx) => ({
      id: `${msg.id}-${idx}`,
      source: msg.from,
      target: recipientId,
      animated: true,
      style: { 
        stroke: getMessageColor(msg.type),
        strokeWidth: 2,
      },
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color: getMessageColor(msg.type),
      },
      label: showLabels ? msg.type : undefined,
      labelStyle: { fontSize: 10 },
      data: { message: msg },
    }));

    setEdges(prev => {
      // Keep recent edges, remove old ones
      const recentEdges = prev.slice(-20);
      return [...recentEdges, ...newEdges];
    });
  }, [showLabels]);

  // Update agent positions
  const updateAgentPositions = useCallback((agents: AgentNode[]) => {
    const newNodes: Node[] = agents.map((agent, idx) => ({
      id: agent.id,
      type: 'custom',
      position: { x: agent.x, y: agent.y },
      data: {
        name: agent.name,
        agentType: agent.type,
        status: agent.status,
      },
      sourcePosition: Position.Right,
      targetPosition: Position.Left,
    }));

    setNodes(newNodes);
  }, []);

  // Get color based on message type
  const getMessageColor = (type: string): string => {
    const colors: Record<string, string> = {
      TaskAssigned: '#3498DB',
      TaskCompleted: '#2ECC71',
      Proposal: '#E74C3C',
      Vote: '#F39C12',
      PatternShare: '#9B59B6',
      HelpRequest: '#E67E22',
      Handoff: '#1ABC9C',
      Progress: '#95A5A6',
    };
    return colors[type] || '#BDC3C7';
  };

  // Clear graph
  const clearGraph = () => {
    setEdges([]);
    setMessageHistory([]);
  };

  // Export graph data
  const exportGraph = () => {
    const data = {
      nodes,
      edges,
      messages: messageHistory,
      timestamp: Date.now(),
    };
    
    const blob = new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `communication-graph-${Date.now()}.json`;
    a.click();
    URL.revokeObjectURL(url);
  };

  // Memoized stats
  const stats = useMemo(() => {
    const typeCount: Record<string, number> = {};
    messageHistory.forEach(msg => {
      typeCount[msg.type] = (typeCount[msg.type] || 0) + 1;
    });
    
    return {
      totalMessages: messageHistory.length,
      messageTypes: typeCount,
      activeEdges: edges.length,
      activeAgents: nodes.length,
    };
  }, [messageHistory, edges, nodes]);

  return (
    <div style={{ height, display: 'flex', flexDirection: 'column' }}>
      {/* Toolbar */}
      <div style={{ 
        padding: '10px', 
        background: '#2C3E50', 
        color: '#fff',
        display: 'flex',
        gap: '10px',
        alignItems: 'center',
      }}>
        <button 
          onClick={() => setIsPaused(!isPaused)}
          style={{
            padding: '6px 12px',
            background: isPaused ? '#E74C3C' : '#27AE60',
            color: '#fff',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          {isPaused ? '▶ Resume' : '⏸ Pause'}
        </button>
        
        <button 
          onClick={clearGraph}
          style={{
            padding: '6px 12px',
            background: '#E74C3C',
            color: '#fff',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          🗑 Clear
        </button>
        
        <button 
          onClick={exportGraph}
          style={{
            padding: '6px 12px',
            background: '#3498DB',
            color: '#fff',
            border: 'none',
            borderRadius: '4px',
            cursor: 'pointer',
          }}
        >
          💾 Export
        </button>
        
        <label style={{ display: 'flex', alignItems: 'center', gap: '5px', marginLeft: 'auto' }}>
          <input 
            type="checkbox" 
            checked={showLabels}
            onChange={(e) => setShowLabels(e.target.checked)}
          />
          Show Labels
        </label>
      </div>

      {/* Stats Bar */}
      <div style={{ 
        padding: '8px', 
        background: '#34495E', 
        color: '#fff',
        fontSize: '12px',
        display: 'flex',
        gap: '20px',
      }}>
        <span>📊 Messages: {stats.totalMessages}</span>
        <span>🔗 Active Edges: {stats.activeEdges}</span>
        <span>👥 Agents: {stats.activeAgents}</span>
        <div style={{ marginLeft: 'auto', display: 'flex', gap: '10px' }}>
          {Object.entries(stats.messageTypes).slice(0, 5).map(([type, count]) => (
            <span key={type} style={{ 
              padding: '2px 6px', 
              background: getMessageColor(type),
              borderRadius: '4px',
              fontSize: '10px',
            }}>
              {type}: {count}
            </span>
          ))}
        </div>
      </div>

      {/* Graph */}
      <div style={{ flex: 1 }}>
        <ReactFlow
          nodes={nodes}
          edges={edges}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          nodeTypes={nodeTypes}
          fitView
          attributionPosition="bottom-left"
        >
          <Background color="#ECF0F1" gap={16} />
          <Controls />
          <MiniMap 
            nodeColor={(node) => nodeColors[node.data?.agentType] || nodeColors.default}
            maskColor="rgba(0, 0, 0, 0.1)"
          />
        </ReactFlow>
      </div>

      {/* Message Details Panel */}
      {selectedMessage && (
        <div style={{
          position: 'absolute',
          bottom: '10px',
          right: '10px',
          width: '300px',
          background: '#fff',
          borderRadius: '8px',
          boxShadow: '0 4px 12px rgba(0,0,0,0.15)',
          padding: '15px',
          maxHeight: '300px',
          overflow: 'auto',
        }}>
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <h4 style={{ margin: 0 }}>Message Details</h4>
            <button onClick={() => setSelectedMessage(null)}>✕</button>
          </div>
          <pre style={{ fontSize: '11px', marginTop: '10px' }}>
            {JSON.stringify(selectedMessage, null, 2)}
          </pre>
        </div>
      )}

      {/* Legend */}
      <div style={{
        position: 'absolute',
        bottom: '10px',
        left: '10px',
        background: 'rgba(255,255,255,0.95)',
        borderRadius: '8px',
        padding: '10px',
        fontSize: '11px',
        boxShadow: '0 2px 8px rgba(0,0,0,0.1)',
      }}>
        <div style={{ fontWeight: 'bold', marginBottom: '8px' }}>Agent Types</div>
        {Object.entries(nodeColors).slice(0, 6).map(([type, color]) => (
          <div key={type} style={{ display: 'flex', alignItems: 'center', gap: '6px', marginBottom: '4px' }}>
            <div style={{ width: '12px', height: '12px', background: color, borderRadius: '2px' }} />
            <span>{type}</span>
          </div>
        ))}
      </div>
    </div>
  );
};

export default CommunicationGraph;
