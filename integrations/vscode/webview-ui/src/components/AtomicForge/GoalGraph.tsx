import React, { useCallback, useEffect } from 'react';
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

  useEffect(() => {
    fetchGraph();
    
    // Polling semplice per refresh (in futuro useremo eventi push)
    const interval = setInterval(fetchGraph, 5000);

    const handler = (event: MessageEvent) => {
      const msg = event.data;
      // Nota: La logica esatta di gestione messaggi dipende da come extension.ts inoltra
      // Assumiamo che estensione inoltri il risultato grezzo
      if (msg.type === 'mcpResponse' && msg.result && !msg.error) {
         try {
            const contentArray = msg.result.content;
            if (Array.isArray(contentArray) && contentArray.length > 0) {
                const text = contentArray[0].text;
                const graphData = JSON.parse(text);
                
                // Aggiorna solo se diverso per evitare re-render loop
                // (Qui semplificato, sovrascriviamo)
                if (graphData.nodes) setNodes(graphData.nodes);
                if (graphData.edges) setEdges(graphData.edges);
            }
         } catch (e) {
             console.error("Graph parse error", e);
         }
      }
    };
    
    window.addEventListener('message', handler);
    return () => {
        window.removeEventListener('message', handler);
        clearInterval(interval);
    };
  }, [fetchGraph, setNodes, setEdges]);

  return (
    <div style={{ width: '100%', height: '100%', backgroundColor: '#1e1e1e' }}>
      <ReactFlow
        nodes={nodes}
        edges={edges}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        fitView
      >
        <Controls />
        <MiniMap style={{ backgroundColor: '#333' }} />
        <Background gap={16} size={1} color="#555" />
      </ReactFlow>
    </div>
  );
}
