import React from 'react';
import AlignmentGauge from '../Alignment/AlignmentGauge';
import GoalTree from '../Goals/GoalTree';
import MessageList from './MessageList';
import ChatInput from './ChatInput';

export default function ChatPanel() {
    return (
        <div style={{
            display: 'flex',
            flexDirection: 'column',
            height: '100%',
            overflow: 'hidden',
        }}>
            <AlignmentGauge />
            <GoalTree />
            <MessageList />
            <ChatInput />

            <style>{`
                @keyframes blink {
                    50% { opacity: 0; }
                }
                pre {
                    background-color: var(--vscode-textCodeBlock-background);
                    padding: 8px;
                    border-radius: 4px;
                    overflow-x: auto;
                    font-size: 12px;
                }
                code {
                    font-family: var(--vscode-editor-font-family);
                    font-size: 12px;
                }
                p { margin: 4px 0; }
                ul, ol { padding-left: 20px; margin: 4px 0; }
                a { color: var(--vscode-textLink-foreground); }
                img { max-width: 100%; }
                table {
                    border-collapse: collapse;
                    width: 100%;
                }
                th, td {
                    border: 1px solid var(--vscode-panel-border);
                    padding: 4px 8px;
                    text-align: left;
                }
                blockquote {
                    border-left: 3px solid var(--vscode-textLink-foreground);
                    padding-left: 12px;
                    margin: 4px 0;
                    color: var(--vscode-descriptionForeground);
                }
            `}</style>
        </div>
    );
}
