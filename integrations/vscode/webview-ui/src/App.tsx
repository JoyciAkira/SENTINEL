import React, { useState } from 'react';
import ChatPanel from './components/Chat/ChatPanel';
import GoalGraph from './components/AtomicForge/GoalGraph';
import { useStore } from './state/store';
import { useVSCodeAPI } from './hooks/useVSCodeAPI';
import { useMCPMessages } from './hooks/useMCPMessages';

export default function App() {
    const vscodeApi = useVSCodeAPI();
    useMCPMessages(vscodeApi);
    const connected = useStore((s) => s.connected);
    const [activeTab, setActiveTab] = useState<'chat' | 'forge'>('chat');

    return (
        <div style={{
            height: '100vh',
            display: 'flex',
            flexDirection: 'column',
            fontFamily: 'var(--vscode-font-family)',
            fontSize: 'var(--vscode-font-size)',
            color: 'var(--vscode-foreground)',
            backgroundColor: 'var(--vscode-sideBar-background)',
        }}>
            {!connected && (
                <div style={{
                    padding: '12px',
                    backgroundColor: 'var(--vscode-inputValidation-warningBackground)',
                    color: 'var(--vscode-inputValidation-warningForeground)',
                    borderBottom: '1px solid var(--vscode-inputValidation-warningBorder)',
                    textAlign: 'center',
                    fontSize: '12px',
                }}>
                    Sentinel not connected. Install sentinel-cli and restart.
                </div>
            )}
            
            <div style={{
                display: 'flex',
                borderBottom: '1px solid var(--vscode-panel-border)',
                backgroundColor: 'var(--vscode-editor-background)'
            }}>
                <button 
                    onClick={() => setActiveTab('chat')}
                    style={{
                        padding: '10px 20px',
                        background: 'none',
                        border: 'none',
                        borderBottom: activeTab === 'chat' ? '2px solid var(--vscode-panelTitle-activeBorder)' : 'none',
                        color: activeTab === 'chat' ? 'var(--vscode-panelTitle-activeForeground)' : 'var(--vscode-panelTitle-inactiveForeground)',
                        cursor: 'pointer',
                        fontFamily: 'inherit'
                    }}
                >
                    Chat
                </button>
                <button 
                    onClick={() => setActiveTab('forge')}
                    style={{
                        padding: '10px 20px',
                        background: 'none',
                        border: 'none',
                        borderBottom: activeTab === 'forge' ? '2px solid var(--vscode-panelTitle-activeBorder)' : 'none',
                        color: activeTab === 'forge' ? 'var(--vscode-panelTitle-activeForeground)' : 'var(--vscode-panelTitle-inactiveForeground)',
                        cursor: 'pointer',
                        fontFamily: 'inherit'
                    }}
                >
                    Atomic Forge (Graph)
                </button>
            </div>

            <div style={{ flex: 1, overflow: 'hidden', display: 'flex', flexDirection: 'column' }}>
                {activeTab === 'chat' ? <ChatPanel /> : <GoalGraph />}
            </div>
        </div>
    );
}