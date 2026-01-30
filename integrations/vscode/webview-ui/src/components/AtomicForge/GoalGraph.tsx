import React, { useCallback, useEffect, useState } from 'react';
import ReactFlow, {
  MiniMap,
  Controls,
  Background,
  useNodesState,
  useEdgesState,
} from 'reactflow';
import 'reactflow/dist/style.css';
import { useVSCodeAPI } from '../../hooks/useVSCodeAPI';

export default function GoalGraph() {
  const [nodes, setNodes, onNodesChange] = useNodesState([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState([]);
  const [selectedNode, setSelectedNode] = useState<any>(null);
  const [loading, setLoading] = useState(false);
  const vscodeApi = useVSCodeAPI();

  const fetchGraph = useCallback(() => {
    vscodeApi.postMessage({
      command: 'mcpRequest',
      method: 'tools/call',
      params: {
        name: 'get_goal_graph',
        arguments: {}
      }
    });
  }, [vscodeApi]);

  const decomposeGoal = (goalId: string) => {
    setLoading(true);
    vscodeApi.postMessage({
      command: 'mcpRequest',
      method: 'tools/call',
      params: {
        name: 'decompose_goal',
        arguments: { goal_id: goalId }
      }
    });
  };

  useEffect(() => {
    fetchGraph();
    const interval = setInterval(fetchGraph, 10000);

    const handler = (event: MessageEvent) => {
      const msg = event.data;
      if (msg.type === 'mcpResponse' && msg.result && !msg.error) {
         try {
            const text = msg.result.content?.[0]?.text;
            if (!text) return;

            if (text.includes("ATOMIC DECOMPOSITION SUCCESS")) {
                setLoading(false);
                fetchGraph();
                return;
            }

            const graphData = JSON.parse(text);
            if (graphData.nodes) {
                const styledNodes = graphData.nodes.map((n: any) => ({
                    ...n,
                    style: {
                        background: n.data.status === 'Completed' ? '#22863a' : 
                                   n.data.status === 'InProgress' ? '#005cc5' : 
                                   n.id === 'root' ? '#6f42c1' : '#333',
                        color: '#fff',
                        borderRadius: '8px',
                        border: '1px solid #555',
                        padding: '10px',
                        fontSize: '11px',
                        width: 180,
                        textAlign: 'center'
                    }
                }));
                setNodes(styledNodes);
            }
            if (graphData.edges) setEdges(graphData.edges);
         } catch (e) {
             // Handle non-JSON responses
         }
      }
    };
    
    window.addEventListener('message', handler);
    return () => {
        window.removeEventListener('message', handler);
        clearInterval(interval);
    };
  }, [fetchGraph, setNodes, setEdges]);

  const onNodeClick = (_: any, node: any) => {
    setSelectedNode(node);
  };

  return (
    <div style={{ width: '100%', height: '100%', position: 'relative', backgroundColor: '#1e1e1e' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeClick={onNodeClick}
        fitView
      >
        <Controls />
        <MiniMap style={{ backgroundColor: '#333' }} />
        <Background gap={16} size={1} color="#444" />
      </ReactFlow>

      {selectedNode && (
        <div style={{
            position: 'absolute',
            right: 0,
            top: 0,
            bottom: 0,
            width: '280px',
            backgroundColor: '#252526',
            borderLeft: '1px solid #444',
            padding: '20px',
            zIndex: 10,
            color: '#ccc',
            display: 'flex',
            flexDirection: 'column',
            boxShadow: '-5px 0 15px rgba(0,0,0,0.3)'
        }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '20px' }}>
                <h3 style={{ margin: 0, color: '#fff', fontSize: '14px' }}>Goal Insights</h3>
                <button onClick={() => setSelectedNode(null)} style={{ background: 'none', border: 'none', color: '#888', cursor: 'pointer', fontSize: '18px' }}>&times;</button>
            </div>
            
            <div style={{ marginBottom: '20px' }}>
                <label style={{ fontSize: '9px', textTransform: 'uppercase', color: '#888', fontWeight: 'bold', letterSpacing: '1px' }}>Description</label>
                <div style={{ color: '#eee', marginTop: '8px', fontSize: '13px', lineHeight: '1.4' }}>{selectedNode.data.label}</div>
            </div>

            <div style={{ marginBottom: '20px' }}>
                <label style={{ fontSize: '9px', textTransform: 'uppercase', color: '#888', fontWeight: 'bold', letterSpacing: '1px' }}>Status</label>
                <div style={{ marginTop: '8px' }}>
                    <span style={{ 
                        padding: '3px 10px', 
                        borderRadius: '12px', 
                        backgroundColor: selectedNode.data.status === 'Completed' ? '#22863a' : '#444',
                        fontSize: '10px',
                        color: '#fff'
                    }}>
                        {selectedNode.data.status || 'Active'}
                    </span>
                </div>
            </div>

            {selectedNode.id !== 'root' && (
                <button 
                    disabled={loading}
                    onClick={() => decomposeGoal(selectedNode.id)}
                    style={{
                        marginTop: 'auto',
                        padding: '12px',
                        backgroundColor: loading ? '#444' : '#007acc',
                        color: '#fff',
                        border: 'none',
                        borderRadius: '4px',
                        cursor: loading ? 'not-allowed' : 'pointer',
                        fontWeight: 'bold',
                        fontSize: '12px',
                        transition: 'background 0.2s'
                    }}
                >
                    {loading ? 'Slicing Goal...' : 'Atomic Decomposition'}
                </button>
            )}
        </div>
      )}
    </div>
  );
}