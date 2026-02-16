/**
 * LivePreviewProvider - Webview panel for real-time development preview
 * Provides seamless integration between Sentinel and local dev servers
 */

import * as vscode from 'vscode';
import {
  DevServer,
  PreviewPanelState,
  ViewportMode,
  VIEWPORT_CONFIGS,
  LivePreviewConfig,
  DEFAULT_PREVIEW_CONFIG,
  FileChangeEvent,
  PreviewMessage,
  InitMessage,
  UrlChangeMessage,
  ViewportChangeMessage
} from '../shared/livePreviewTypes';
import { devServerDetector } from './devServerDetector';

export class LivePreviewProvider implements vscode.WebviewViewProvider {
  /** Must match the view id in package.json "views.sentinel-explorer" */
  public static readonly viewType = "sentinel-live-preview";
  
  private webviewView: vscode.WebviewView | undefined;
  private state: PreviewPanelState = {
    server: null,
    viewport: 'desktop',
    isLoading: false,
    lastError: null,
    refreshCount: 0,
    lastRefresh: 0
  };
  private config: LivePreviewConfig;
  private disposables: vscode.Disposable[] = [];
  private fileWatcher: vscode.FileSystemWatcher | undefined;

  constructor(
    private context: vscode.ExtensionContext,
    config: Partial<LivePreviewConfig> = {}
  ) {
    this.config = { ...DEFAULT_PREVIEW_CONFIG, ...config };
  }

  /**
   * Called when webview is first shown
   */
  resolveWebviewView(
    webviewView: vscode.WebviewView,
    context: vscode.WebviewViewResolveContext,
    token: vscode.CancellationToken
  ): void | Thenable<void> {
    this.webviewView = webviewView;

    // Configure webview
    webviewView.webview.options = {
      enableScripts: true,
      localResourceRoots: [this.context.extensionUri]
    };

    // Set initial HTML
    this.updateContent();

    // Handle messages from webview
    this.disposables.push(
      webviewView.webview.onDidReceiveMessage(this.handleWebviewMessage.bind(this))
    );

    // Handle visibility changes
    this.disposables.push(
      webviewView.onDidChangeVisibility(() => {
        if (webviewView.visible && this.state.server) {
          this.refresh();
        }
      })
    );

    // Auto-detect and start if configured
    if (this.config.autoStart) {
      this.autoDetectAndStart();
    }

    // Setup file watching for auto-refresh
    this.setupFileWatcher();
  }

  /**
   * Auto-detect dev server and start preview
   */
  private async autoDetectAndStart(): Promise<void> {
    try {
      this.updateState({ isLoading: true, lastError: null });
      
      const server = await devServerDetector.quickDetect();
      
      if (server) {
        await this.startPreview(server);
      } else {
        // Show helpful message when no server found
        this.updateState({ 
          isLoading: false, 
          lastError: 'No dev server detected. Start your dev server to see live preview.'
        });
      }
    } catch (error) {
      this.updateState({
        isLoading: false,
        lastError: error instanceof Error ? error.message : 'Failed to detect server'
      });
    }
  }

  /**
   * Start preview with specific server
   */
  async startPreview(server: DevServer): Promise<void> {
    this.updateState({ 
      server, 
      isLoading: true,
      lastError: null 
    });

    // Send init message to webview
    await this.postMessage({
      type: 'init',
      payload: {
        url: `http://localhost:${server.port}${server.path}`,
        viewport: this.state.viewport,
        title: this.getServerDisplayName(server)
      }
    } as InitMessage);

    // Mark as loaded after a short delay
    setTimeout(() => {
      this.updateState({ isLoading: false });
    }, 500);
  }

  /**
   * Stop current preview
   */
  stopPreview(): void {
    this.updateState({
      server: null,
      isLoading: false,
      lastError: null
    });
    this.updateContent();
  }

  /**
   * Refresh the preview
   */
  refresh(): void {
    if (!this.state.server) return;

    this.updateState({
      refreshCount: this.state.refreshCount + 1,
      lastRefresh: Date.now()
    });

    this.postMessage({
      type: 'refresh'
    });
  }

  /**
   * Change viewport mode
   */
  async changeViewport(viewport: ViewportMode): Promise<void> {
    this.updateState({ viewport });
    
    await this.postMessage({
      type: 'viewport-change',
      payload: {
        viewport,
        dimensions: VIEWPORT_CONFIGS[viewport]
      }
    } as ViewportChangeMessage);
  }

  /**
   * Update URL (for navigation)
   */
  async changeUrl(url: string): Promise<void> {
    await this.postMessage({
      type: 'url-change',
      payload: { url }
    } as UrlChangeMessage);
  }

  /**
   * Handle file changes for auto-refresh
   */
  handleFileChange(event: FileChangeEvent): void {
    if (!this.config.autoSync || !this.state.server) return;

    // Debounce refresh
    clearTimeout(this.refreshTimeout);
    this.refreshTimeout = setTimeout(() => {
      this.refresh();
    }, this.config.refreshDelay);
  }

  private refreshTimeout: ReturnType<typeof setTimeout> | undefined;

  /**
   * Setup file watcher for auto-refresh
   */
  private setupFileWatcher(): void {
    // Watch for file changes in workspace
    const watcher = vscode.workspace.createFileSystemWatcher(
      '**/*.{html,css,scss,js,ts,jsx,tsx,vue,svelte}'
    );

    this.disposables.push(
      watcher.onDidChange(uri => this.handleFileChange({
        uri: uri.toString(),
        type: 'modified',
        timestamp: Date.now()
      }))
    );

    this.disposables.push(
      watcher.onDidCreate(uri => this.handleFileChange({
        uri: uri.toString(),
        type: 'created',
        timestamp: Date.now()
      }))
    );

    this.fileWatcher = watcher;
  }

  /**
   * Handle messages from webview
   */
  private handleWebviewMessage(message: PreviewMessage): void {
    switch (message.type) {
      case 'ready':
        // Webview is ready, send initial state if we have a server
        if (this.state.server) {
          this.startPreview(this.state.server);
        }
        break;

      case 'error':
        this.updateState({
          lastError: (message as any).payload?.message || 'Unknown error'
        });
        break;

      case 'health-check':
        // Webview is checking health
        this.postMessage({ type: 'health-check', payload: { healthy: true } });
        break;
    }
  }

  /**
   * Post message to webview
   */
  private postMessage(message: PreviewMessage): Thenable<boolean> {
    if (!this.webviewView) return Promise.resolve(false);
    return this.webviewView.webview.postMessage(message);
  }

  /**
   * Update panel state
   */
  private updateState(updates: Partial<PreviewPanelState>): void {
    this.state = { ...this.state, ...updates };
    this.updateContent();
  }

  /**
   * Get display name for server
   */
  private getServerDisplayName(server: DevServer): string {
    const typeNames: Record<string, string> = {
      vite: 'Vite',
      nextjs: 'Next.js',
      nuxt: 'Nuxt',
      'react-scripts': 'React',
      'vue-cli': 'Vue CLI',
      angular: 'Angular',
      sveltekit: 'SvelteKit',
      astro: 'Astro',
      remix: 'Remix',
      gatsby: 'Gatsby',
      parcel: 'Parcel',
      webpack: 'Webpack',
      custom: 'Dev Server'
    };

    return `${typeNames[server.type] || server.type} (localhost:${server.port})`;
  }

  /**
   * Update webview HTML content
   */
  private updateContent(): void {
    if (!this.webviewView) return;
    this.webviewView.webview.html = this.getHtml();
  }

  /**
   * Generate HTML for webview
   */
  private getHtml(): string {
    const webview = this.webviewView!.webview;
    const scriptUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, 'out', 'previewPanel.js')
    );
    const styleUri = webview.asWebviewUri(
      vscode.Uri.joinPath(this.context.extensionUri, 'out', 'previewPanel.css')
    );

    const nonce = this.getNonce();

    return `<!DOCTYPE html>
      <html lang="en">
      <head>
        <meta charset="UTF-8">
        <meta name="viewport" content="width=device-width, initial-scale=1.0">
        <meta http-equiv="Content-Security-Policy" content="default-src 'none'; frame-src http: https:; script-src 'nonce-${nonce}' 'unsafe-inline'; style-src ${webview.cspSource} 'unsafe-inline'; img-src ${webview.cspSource} http: https:;">
        <link href="${styleUri}" rel="stylesheet">
        <title>Sentinel Live Preview</title>
      </head>
      <body>
        <div id="root"></div>
        <script nonce="${nonce}">
          window.initialState = ${JSON.stringify(this.state)};
          window.config = ${JSON.stringify(this.config)};
        </script>
        <script nonce="${nonce}" src="${scriptUri}"></script>
      </body>
      </html>
    `;
  }

  /**
   * Generate nonce for CSP
   */
  private getNonce(): string {
    let text = '';
    const possible = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    for (let i = 0; i < 32; i++) {
      text += possible.charAt(Math.floor(Math.random() * possible.length));
    }
    return text;
  }

  /**
   * Get current state
   */
  getState(): PreviewPanelState {
    return { ...this.state };
  }

  /**
   * Dispose resources
   */
  dispose(): void {
    this.disposables.forEach(d => d.dispose());
    this.fileWatcher?.dispose();
  }
}
