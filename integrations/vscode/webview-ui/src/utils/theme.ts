// VS Code CSS custom properties integration
// These variables are automatically available in webviews

export const colors = {
    background: 'var(--vscode-sideBar-background)',
    foreground: 'var(--vscode-foreground)',
    inputBg: 'var(--vscode-input-background)',
    inputFg: 'var(--vscode-input-foreground)',
    inputBorder: 'var(--vscode-input-border)',
    buttonBg: 'var(--vscode-button-background)',
    buttonFg: 'var(--vscode-button-foreground)',
    buttonHoverBg: 'var(--vscode-button-hoverBackground)',
    secondaryButtonBg: 'var(--vscode-button-secondaryBackground)',
    secondaryButtonFg: 'var(--vscode-button-secondaryForeground)',
    border: 'var(--vscode-panel-border)',
    accentFg: 'var(--vscode-textLink-foreground)',
    errorFg: 'var(--vscode-errorForeground)',
    warningFg: 'var(--vscode-editorWarning-foreground)',
    successFg: 'var(--vscode-testing-iconPassed)',
    badgeBg: 'var(--vscode-badge-background)',
    badgeFg: 'var(--vscode-badge-foreground)',
    descriptionFg: 'var(--vscode-descriptionForeground)',
} as const;
