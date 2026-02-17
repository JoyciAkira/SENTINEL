// ProviderConfigPanel.tsx - Secure API key configuration UI
// Uses VSCode SecretStorage for 100% secure storage

import React, { useState, useEffect, useCallback } from 'react';
import { useVSCodeAPI } from '../../hooks/useVSCodeAPI';
import './ProviderConfigPanel.css';

interface Provider {
  id: string;
  name: string;
  description: string;
  keyEnvVar: string;
  defaultModel: string;
  isConfigured: boolean;
  isEnabled: boolean;
}

interface ProviderConfigState {
  providers: Provider[];
  selectedProvider: string | null;
  apiKey: string;
  isLoading: boolean;
  message: string;
  messageType: 'success' | 'error' | 'info';
  showKey: boolean;
}

const PROVIDER_INFO: Record<string, { url: string; icon: string }> = {
  openrouter: {
    url: 'https://openrouter.ai/keys',
    icon: '🌐',
  },
  openai: {
    url: 'https://platform.openai.com/api-keys',
    icon: '🤖',
  },
  anthropic: {
    url: 'https://console.anthropic.com/settings/keys',
    icon: '🧠',
  },
  google: {
    url: 'https://aistudio.google.com/app/apikey',
    icon: '🔍',
  },
  groq: {
    url: 'https://console.groq.com/keys',
    icon: '⚡',
  },
  ollama: {
    url: 'https://ollama.com',
    icon: '📦',
  },
};

export const ProviderConfigPanel: React.FC = () => {
  const vscode = useVSCodeAPI();
  const [state, setState] = useState<ProviderConfigState>({
    providers: [],
    selectedProvider: null,
    apiKey: '',
    isLoading: true,
    message: '',
    messageType: 'info',
    showKey: false,
  });

  // Load providers on mount
  useEffect(() => {
    vscode.postMessage({ type: 'getProviders' });

    const handleMessage = (event: MessageEvent) => {
      const message = event.data;
      
      if (message.type === 'providersList') {
        setState(prev => ({
          ...prev,
          providers: message.providers,
          isLoading: false,
        }));
      } else if (message.type === 'providerToggled') {
        setState(prev => ({
          ...prev,
          message: `${message.enabled ? '✅ Enabled' : '⏸️ Disabled'} ${message.provider}`,
          messageType: 'info',
          isLoading: false,
        }));
        vscode.postMessage({ type: 'getProviders' });
      } else if (message.type === 'providerTestResult') {
        setState(prev => ({
          ...prev,
          message: message.message ?? `Provider test ${message.success ? 'passed' : 'failed'}`,
          messageType: message.success ? 'success' : 'error',
          isLoading: false,
        }));
      } else if (message.type === 'providerSaved') {
        setState(prev => ({
          ...prev,
          apiKey: '',
          selectedProvider: null,
          message: `✅ ${message.provider} API key saved securely`,
          messageType: 'success',
          isLoading: false,
        }));
        // Refresh list
        vscode.postMessage({ type: 'getProviders' });
      } else if (message.type === 'providerDeleted') {
        setState(prev => ({
          ...prev,
          message: `🗑️ ${message.provider} API key removed`,
          messageType: 'info',
        }));
        vscode.postMessage({ type: 'getProviders' });
      } else if (message.type === 'error') {
        setState(prev => ({
          ...prev,
          message: `❌ ${message.error}`,
          messageType: 'error',
          isLoading: false,
        }));
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, [vscode]);

  const saveApiKey = useCallback(() => {
    if (!state.selectedProvider || !state.apiKey.trim()) {
      setState(prev => ({
        ...prev,
        message: '❌ Please select a provider and enter API key',
        messageType: 'error',
      }));
      return;
    }

    setState(prev => ({ ...prev, isLoading: true }));

    vscode.postMessage({
      type: 'saveProviderKey',
      provider: state.selectedProvider,
      apiKey: state.apiKey.trim(),
    });
  }, [state.selectedProvider, state.apiKey]);

  const deleteApiKey = useCallback((providerId: string) => {
    if (confirm(`Are you sure you want to remove the ${providerId} API key?`)) {
      vscode.postMessage({
        type: 'deleteProviderKey',
        provider: providerId,
      });
    }
  }, []);

  const exportEnvScript = useCallback(() => {
    vscode.postMessage({ type: 'exportEnvScript' });
    setState(prev => ({
      ...prev,
      message: '📋 Environment script exported to clipboard',
      messageType: 'success',
    }));
  }, []);

  const testConnection = useCallback((providerId: string) => {
    setState(prev => ({ ...prev, isLoading: true }));
    vscode.postMessage({
      type: 'testProviderConnection',
      provider: providerId,
    });
    setState(prev => ({
      ...prev,
      message: `🧪 Testing ${providerId} connection...`,
      messageType: 'info',
    }));
  }, []);

  const toggleProvider = useCallback((providerId: string, enabled: boolean) => {
    setState(prev => ({ ...prev, isLoading: true }));
    vscode.postMessage({
      type: 'toggleProviderEnabled',
      provider: providerId,
      enabled,
    });
  }, []);

  const configuredCount = state.providers.filter(p => p.isConfigured).length;

  return (
    <div className="provider-config-panel">
      {/* Header */}
      <div className="config-header">
        <h2>🔐 Provider Configuration</h2>
        <p className="config-subtitle">
          Securely store API keys using VSCode's encrypted SecretStorage
        </p>
        <div className="security-badge">
          <span className="lock-icon">🔒</span>
          <span>100% Secure - Encrypted at rest using OS keychain</span>
        </div>
      </div>

      {/* Status Bar */}
      <div className="status-bar">
        <span className="status-text">
          {configuredCount} of {state.providers.length} providers configured
        </span>
        {configuredCount > 0 && (
          <button className="export-btn" onClick={exportEnvScript}>
            📋 Export Env Script
          </button>
        )}
      </div>

      {/* Message */}
      {state.message && (
        <div className={`message ${state.messageType}`}>
          {state.message}
        </div>
      )}

      {/* Providers List */}
      <div className="providers-list">
        {state.providers.map(provider => {
          const info = PROVIDER_INFO[provider.id];
          return (
            <div
              key={provider.id}
              className={`provider-card ${provider.isConfigured ? 'configured' : ''} ${
                state.selectedProvider === provider.id ? 'selected' : ''
              }`}
              onClick={() => setState(prev => ({
                ...prev,
                selectedProvider: provider.id,
                apiKey: '',
              }))}
            >
              <div className="provider-icon">{info?.icon || '🔧'}</div>
              <div className="provider-info">
                <div className="provider-name">
                  {provider.name}
                  {provider.isConfigured && (
                    <span className="configured-badge">✓</span>
                  )}
                  {!provider.isEnabled && (
                    <span className="disabled-badge">disabled</span>
                  )}
                </div>
                <div className="provider-description">{provider.description}</div>
                <div className="provider-model">Model: {provider.defaultModel}</div>
              </div>
              <div className="provider-actions">
                <button
                  className={`toggle-btn ${provider.isEnabled ? 'enabled' : ''}`}
                  onClick={(e) => {
                    e.stopPropagation();
                    toggleProvider(provider.id, !provider.isEnabled);
                  }}
                  title={provider.isEnabled ? 'Disable provider' : 'Enable provider'}
                >
                  {provider.isEnabled ? 'ON' : 'OFF'}
                </button>
                {provider.isConfigured ? (
                  <>
                    <button
                      className="test-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        testConnection(provider.id);
                      }}
                      title="Test connection"
                    >
                      🧪
                    </button>
                    <button
                      className="delete-btn"
                      onClick={(e) => {
                        e.stopPropagation();
                        deleteApiKey(provider.id);
                      }}
                      title="Remove API key"
                    >
                      🗑️
                    </button>
                  </>
                ) : (
                  <a
                    href={info?.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="get-key-link"
                    onClick={(e) => e.stopPropagation()}
                  >
                    Get Key →
                  </a>
                )}
              </div>
            </div>
          );
        })}
      </div>

      {/* Configuration Form */}
      {state.selectedProvider && state.selectedProvider !== 'openai_auth' && (
        <div className="config-form">
          <h3>Configure {state.providers.find(p => p.id === state.selectedProvider)?.name}</h3>
          
          <div className="input-group">
            <label>API Key:</label>
            <div className="api-key-input">
              <input
                type={state.showKey ? 'text' : 'password'}
                value={state.apiKey}
                onChange={(e) => setState(prev => ({ ...prev, apiKey: e.target.value }))}
                placeholder={`Enter ${state.selectedProvider} API key`}
                className="key-input"
              />
              <button
                className="toggle-visibility"
                onClick={() => setState(prev => ({ ...prev, showKey: !prev.showKey }))}
                title={state.showKey ? 'Hide key' : 'Show key'}
              >
                {state.showKey ? '🙈' : '👁️'}
              </button>
            </div>
            <p className="input-help">
              Your API key is stored securely using VSCode's SecretStorage
              and is never sent to any server.
            </p>
          </div>

          <div className="form-actions">
            <button
              className="cancel-btn"
              onClick={() => setState(prev => ({
                ...prev,
                selectedProvider: null,
                apiKey: '',
              }))}
            >
              Cancel
            </button>
            <button
              className="save-btn"
              onClick={saveApiKey}
              disabled={!state.apiKey.trim() || state.isLoading}
            >
              {state.isLoading ? 'Saving...' : '💾 Save Securely'}
            </button>
          </div>
        </div>
      )}

      {/* Instructions */}
      <div className="instructions">
        <h4>🔐 Security Information</h4>
        <ul>
          <li>✅ API keys are encrypted using your OS keychain (Keychain on macOS, Credential Manager on Windows, Secret Service on Linux)</li>
          <li>✅ Keys are never stored in plain text or synced to cloud</li>
          <li>✅ Only this VSCode instance can access your keys</li>
          <li>✅ Keys are never sent to SENTINEL servers</li>
          <li>✅ You can remove keys at any time</li>
        </ul>
        
        <h4>📋 How to Use</h4>
        <ol>
          <li>Click on a provider above</li>
          <li>Click "Get Key →" to open the provider's website</li>
          <li>Create an API key on the provider's website</li>
          <li>Copy and paste the key here</li>
          <li>Click "Save Securely"</li>
          <li>Done! The swarm will use this provider automatically</li>
        </ol>
      </div>

      {/* Footer */}
      <div className="config-footer">
        <p>
          💡 <strong>Tip:</strong> Configure multiple providers for automatic fallback.
          If one provider fails, SENTINEL will automatically try the next.
        </p>
      </div>
    </div>
  );
};

export default ProviderConfigPanel;
