// SwarmPanel.tsx - Real-time Swarm Visualization
// Visualizza agenti attivi, consensus, e progresso in tempo reale

import React, { useEffect, useState, useCallback } from 'react';
import { useVSCodeAPI } from '../../hooks/useVSCodeAPI';
import './SwarmPanel.css';

// Types matching Rust structures
interface Agent {
  id: string;
  name: string;
  type: string;
  status: 'idle' | 'working' | 'completed' | 'failed';
  progress: number;
  currentTask: string;
  authority: number;
}

interface Proposal {
  id: string;
  title: string;
  status: 'voting' | 'accepted' | 'rejected' | 'timeout';
  votes: {
    approve: number;
    reject: number;
    abstain: number;
  };
}

interface SwarmMessage {
  type: 'agentUpdate' | 'consensusUpdate' | 'proposalUpdate' | 'system';
  payload: any;
}

interface SwarmState {
  agents: Agent[];
  proposals: Proposal[];
  consensusRound: number;
  swarmStatus: 'initializing' | 'running' | 'completed' | 'failed';
  executionTime: number;
}

export const SwarmPanel: React.FC = () => {
  const vscode = useVSCodeAPI();
  const [swarmState, setSwarmState] = useState<SwarmState>({
    agents: [],
    proposals: [],
    consensusRound: 0,
    swarmStatus: 'initializing',
    executionTime: 0,
  });

  const [selectedAgent, setSelectedAgent] = useState<Agent | null>(null);
  const [viewMode, setViewMode] = useState<'grid' | 'list' | 'graph'>('grid');
  const [autoRefresh, setAutoRefresh] = useState(true);

  // Listen for messages from VSCode extension
  useEffect(() => {
    const handleMessage = (event: MessageEvent) => {
      const message = event.data as SwarmMessage;
      
      switch (message.type) {
        case 'agentUpdate':
          updateAgent(message.payload);
          break;
        case 'consensusUpdate':
          updateConsensus(message.payload);
          break;
        case 'proposalUpdate':
          updateProposal(message.payload);
          break;
        case 'system':
          handleSystemMessage(message.payload);
          break;
      }
    };

    window.addEventListener('message', handleMessage);
    
    // Request initial state
    vscode.postMessage({ type: 'getSwarmState' });

    return () => window.removeEventListener('message', handleMessage);
  }, [vscode]);

  const updateAgent = useCallback((agentUpdate: Agent) => {
    setSwarmState(prev => {
      const existingIndex = prev.agents.findIndex(a => a.id === agentUpdate.id);
      
      if (existingIndex >= 0) {
        // Update existing agent
        const newAgents = [...prev.agents];
        newAgents[existingIndex] = { ...newAgents[existingIndex], ...agentUpdate };
        return { ...prev, agents: newAgents };
      } else {
        // Add new agent
        return { ...prev, agents: [...prev.agents, agentUpdate] };
      }
    });
  }, []);

  const updateConsensus = useCallback((update: { round: number }) => {
    setSwarmState(prev => ({
      ...prev,
      consensusRound: update.round,
    }));
  }, []);

  const updateProposal = useCallback((proposal: Proposal) => {
    setSwarmState(prev => {
      const existingIndex = prev.proposals.findIndex(p => p.id === proposal.id);
      
      if (existingIndex >= 0) {
        const newProposals = [...prev.proposals];
        newProposals[existingIndex] = proposal;
        return { ...prev, proposals: newProposals };
      } else {
        return { ...prev, proposals: [...prev.proposals, proposal] };
      }
    });
  }, []);

  const handleSystemMessage = useCallback((payload: { status: string; executionTime?: number }) => {
    setSwarmState(prev => ({
      ...prev,
      swarmStatus: payload.status as any,
      executionTime: payload.executionTime || prev.executionTime,
    }));
  }, []);

  const getAgentIcon = (type: string) => {
    switch (type) {
      case 'AuthArchitect': return '🔷';
      case 'SecurityAuditor': return '🛡️';
      case 'JWTCoder': return '🔑';
      case 'APICoder': return '🔧';
      case 'FrontendCoder': return '🎨';
      case 'DatabaseArchitect': return '🗄️';
      case 'TestWriter': return '✅';
      case 'DocWriter': return '📚';
      case 'ReviewAgent': return '👁️';
      case 'PerformanceOptimizer': return '⚡';
      case 'DevOpsEngineer': return '🚀';
      case 'Manager': return '👔';
      default: return '🤖';
    }
  };

  const getStatusColor = (status: string) => {
    switch (status) {
      case 'working': return '#0e639c'; // Blue
      case 'completed': return '#4ec9b0'; // Green
      case 'failed': return '#f14c4c'; // Red
      case 'idle': return '#6e6e6e'; // Gray
      default: return '#6e6e6e';
    }
  };

  const activeAgents = swarmState.agents.filter(a => a.status === 'working').length;
  const completedAgents = swarmState.agents.filter(a => a.status === 'completed').length;
  const totalAgents = swarmState.agents.length;

  return (
    <div className="swarm-panel">
      {/* Header */}
      <div className="swarm-header">
        <div className="swarm-title">
          <span className="swarm-icon">🐝</span>
          <h2>Sentinel Swarm</h2>
          <span className={`swarm-status ${swarmState.swarmStatus}`}>
            {swarmState.swarmStatus}
          </span>
        </div>
        
        <div className="swarm-stats">
          <div className="stat">
            <span className="stat-value">{totalAgents}</span>
            <span className="stat-label">Agents</span>
          </div>
          <div className="stat active">
            <span className="stat-value">{activeAgents}</span>
            <span className="stat-label">Active</span>
          </div>
          <div className="stat completed">
            <span className="stat-value">{completedAgents}</span>
            <span className="stat-label">Done</span>
          </div>
          <div className="stat">
            <span className="stat-value">{swarmState.consensusRound}</span>
            <span className="stat-label">Rounds</span>
          </div>
        </div>

        <div className="swarm-controls">
          <button
            className={`view-btn ${viewMode === 'grid' ? 'active' : ''}`}
            onClick={() => setViewMode('grid')}
            title="Grid View"
          >
            ⊞
          </button>
          <button
            className={`view-btn ${viewMode === 'list' ? 'active' : ''}`}
            onClick={() => setViewMode('list')}
            title="List View"
          >
            ☰
          </button>
          <button
            className={`view-btn ${viewMode === 'graph' ? 'active' : ''}`}
            onClick={() => setViewMode('graph')}
            title="Graph View"
          >
            ◎
          </button>
          <label className="auto-refresh">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
            />
            Live
          </label>
        </div>
      </div>

      {/* Main Content */}
      <div className={`swarm-content ${viewMode}`}>
        {/* Agents Section */}
        <div className="agents-section">
          <h3>Active Agents</h3>
          
          {viewMode === 'grid' && (
            <div className="agents-grid">
              {swarmState.agents.map(agent => (
                <div
                  key={agent.id}
                  className={`agent-card ${agent.status}`}
                  onClick={() => setSelectedAgent(agent)}
                  style={{ borderColor: getStatusColor(agent.status) }}
                >
                  <div className="agent-header">
                    <span className="agent-icon">{getAgentIcon(agent.type)}</span>
                    <span className="agent-name">{agent.name || agent.type}</span>
                    <span className="agent-authority">{Math.round(agent.authority * 100)}%</span>
                  </div>
                  
                  <div className="agent-task">{agent.currentTask || 'Idle'}</div>
                  
                  <div className="agent-progress">
                    <div className="progress-bar">
                      <div
                        className="progress-fill"
                        style={{
                          width: `${agent.progress}%`,
                          backgroundColor: getStatusColor(agent.status),
                        }}
                      />
                    </div>
                    <span className="progress-text">{Math.round(agent.progress)}%</span>
                  </div>
                  
                  <div className={`agent-status-badge ${agent.status}`}>
                    {agent.status}
                  </div>
                </div>
              ))}
            </div>
          )}

          {viewMode === 'list' && (
            <div className="agents-list">
              {swarmState.agents.map(agent => (
                <div
                  key={agent.id}
                  className={`agent-row ${agent.status}`}
                  onClick={() => setSelectedAgent(agent)}
                >
                  <span className="agent-icon">{getAgentIcon(agent.type)}</span>
                  <span className="agent-name">{agent.name || agent.type}</span>
                  <span className="agent-task">{agent.currentTask || 'Idle'}</span>
                  <div className="agent-progress-bar">
                    <div
                      className="progress-fill"
                      style={{ width: `${agent.progress}%` }}
                    />
                  </div>
                  <span className="agent-progress-text">{Math.round(agent.progress)}%</span>
                  <span className={`status-dot ${agent.status}`} />
                </div>
              ))}
            </div>
          )}

          {viewMode === 'graph' && (
            <div className="agents-graph">
              {/* Simplified graph visualization */}
              <svg viewBox="0 0 400 300" className="graph-svg">
                {swarmState.agents.map((agent, index) => {
                  const angle = (index / swarmState.agents.length) * 2 * Math.PI;
                  const x = 200 + 100 * Math.cos(angle);
                  const y = 150 + 100 * Math.sin(angle);
                  
                  return (
                    <g key={agent.id} className="graph-node">
                      <circle
                        cx={x}
                        cy={y}
                        r="30"
                        fill={getStatusColor(agent.status)}
                        opacity={0.8}
                      />
                      <text x={x} y={y} textAnchor="middle" fill="white" fontSize="12">
                        {getAgentIcon(agent.type)}
                      </text>
                      <text x={x} y={y + 45} textAnchor="middle" fill="#ccc" fontSize="10">
                        {agent.type}
                      </text>
                    </g>
                  );
                })}
                
                {/* Center hub */}
                <circle cx="200" cy="150" r="20" fill="#0e639c" />
                <text x="200" y="155" textAnchor="middle" fill="white" fontSize="12">
                  🐝
                </text>
              </svg>
            </div>
          )}
        </div>

        {/* Consensus Panel */}
        <div className="consensus-section">
          <h3>Consensus ({swarmState.consensusRound} rounds)</h3>
          
          <div className="proposals-list">
            {swarmState.proposals.map(proposal => (
              <div key={proposal.id} className={`proposal-card ${proposal.status}`}>
                <div className="proposal-header">
                  <span className="proposal-title">{proposal.title}</span>
                  <span className={`proposal-status ${proposal.status}`}>
                    {proposal.status}
                  </span>
                </div>
                
                <div className="proposal-votes">
                  <div className="vote-bar">
                    <div
                      className="vote-approve"
                      style={{ width: `${(proposal.votes.approve / 5) * 100}%` }}
                    />
                    <div
                      className="vote-reject"
                      style={{ width: `${(proposal.votes.reject / 5) * 100}%` }}
                    />
                  </div>
                  <div className="vote-counts">
                    <span className="approve-count">✓ {proposal.votes.approve}</span>
                    <span className="reject-count">✗ {proposal.votes.reject}</span>
                    <span className="abstain-count">○ {proposal.votes.abstain}</span>
                  </div>
                </div>
              </div>
            ))}
            
            {swarmState.proposals.length === 0 && (
              <div className="no-proposals">No active proposals</div>
            )}
          </div>
        </div>
      </div>

      {/* Agent Detail Modal */}
      {selectedAgent && (
        <div className="agent-modal" onClick={() => setSelectedAgent(null)}>
          <div className="agent-modal-content" onClick={e => e.stopPropagation()}>
            <div className="modal-header">
              <h3>{getAgentIcon(selectedAgent.type)} {selectedAgent.name || selectedAgent.type}</h3>
              <button className="close-btn" onClick={() => setSelectedAgent(null)}>×</button>
            </div>
            
            <div className="modal-body">
              <div className="detail-row">
                <span className="detail-label">ID:</span>
                <span className="detail-value">{selectedAgent.id}</span>
              </div>
              <div className="detail-row">
                <span className="detail-label">Type:</span>
                <span className="detail-value">{selectedAgent.type}</span>
              </div>
              <div className="detail-row">
                <span className="detail-label">Status:</span>
                <span className={`detail-value status ${selectedAgent.status}`}>
                  {selectedAgent.status}
                </span>
              </div>
              <div className="detail-row">
                <span className="detail-label">Authority:</span>
                <span className="detail-value">{Math.round(selectedAgent.authority * 100)}%</span>
              </div>
              <div className="detail-row">
                <span className="detail-label">Progress:</span>
                <span className="detail-value">{Math.round(selectedAgent.progress)}%</span>
              </div>
              <div className="detail-row">
                <span className="detail-label">Current Task:</span>
                <span className="detail-value">{selectedAgent.currentTask || 'None'}</span>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Footer */}
      <div className="swarm-footer">
        <div className="footer-info">
          <span>⏱️ {swarmState.executionTime}ms</span>
          <span>🔄 {autoRefresh ? 'Live' : 'Paused'}</span>
        </div>
        <div className="footer-actions">
          <button onClick={() => vscode.postMessage({ type: 'pauseSwarm' })}>
            ⏸️ Pause
          </button>
          <button onClick={() => vscode.postMessage({ type: 'stopSwarm' })}>
            ⏹️ Stop
          </button>
        </div>
      </div>
    </div>
  );
};

export default SwarmPanel;