import React from 'react';
import ChatPanel from './components/Chat/ChatPanel';
import { useStore } from './state/store';
import { useVSCodeAPI } from './hooks/useVSCodeAPI';
import { useMCPMessages } from './hooks/useMCPMessages';

export default function App() {
    const vscodeApi = useVSCodeAPI();
    useMCPMessages(vscodeApi);

    const connected = useStore((s) => s.connected);

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
            <ChatPanel />
        </div>
    );
}
