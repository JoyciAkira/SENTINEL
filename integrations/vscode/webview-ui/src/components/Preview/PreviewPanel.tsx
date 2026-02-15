/**
 * PreviewPanel - World-class live preview component
 * Renders development server in iframe with elegant controls
 */

import React, { useEffect, useRef, useState, useCallback } from 'react';
import { 
  Monitor, 
  Smartphone, 
  Tablet, 
  RefreshCw, 
  ExternalLink,
  Maximize2,
  AlertCircle,
  Loader2,
  CheckCircle2
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '../ui/button';
import {
  ViewportMode,
  ViewportDimensions,
  VIEWPORT_CONFIGS,
  PreviewPanelState,
  PreviewMessage,
  LivePreviewConfig
} from '../../../../src/shared/livePreviewTypes';

// Get initial state from window (injected by provider)
declare global {
  interface Window {
    initialState?: PreviewPanelState;
    config?: LivePreviewConfig;
    acquireVsCodeApi?: () => {
      postMessage: (message: unknown) => void;
      getState: () => unknown;
      setState: (state: unknown) => void;
    };
  }
}

interface PreviewPanelProps {
  className?: string;
}

export const PreviewPanel: React.FC<PreviewPanelProps> = ({ className }) => {
  // State
  const [state, setState] = useState<PreviewPanelState>(
    window.initialState || {
      server: null,
      viewport: 'desktop',
      isLoading: false,
      lastError: null,
      refreshCount: 0,
      lastRefresh: 0
    }
  );
  const [iframeUrl, setIframeUrl] = useState<string>('');
  const [iframeKey, setIframeKey] = useState<number>(0);
  const [showError, setShowError] = useState<boolean>(false);
  const [isHovering, setIsHovering] = useState<boolean>(false);
  
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const vscodeApi = useRef<ReturnType<typeof window.acquireVsCodeApi> | null>(null);

  // Initialize VSCode API
  useEffect(() => {
    if (window.acquireVsCodeApi) {
      vscodeApi.current = window.acquireVsCodeApi();
      // Notify extension that webview is ready
      vscodeApi.current.postMessage({ type: 'ready' });
    }
  }, []);

  // Listen for messages from extension
  useEffect(() => {
    const handleMessage = (event: MessageEvent<PreviewMessage>) => {
      const message = event.data;
      
      switch (message.type) {
        case 'init':
          const initPayload = (message as any).payload;
          if (initPayload?.url) {
            setIframeUrl(initPayload.url);
            setState(prev => ({ 
              ...prev, 
              server: { ...prev.server, ...initPayload } as any,
              isLoading: true 
            }));
          }
          break;

        case 'url-change':
          const urlPayload = (message as any).payload;
          if (urlPayload?.url) {
            setIframeUrl(urlPayload.url);
          }
          break;

        case 'viewport-change':
          const vpPayload = (message as any).payload;
          if (vpPayload?.viewport) {
            setState(prev => ({ 
              ...prev, 
              viewport: vpPayload.viewport 
            }));
          }
          break;

        case 'refresh':
          handleRefresh();
          break;

        case 'error':
          const errorPayload = (message as any).payload;
          setState(prev => ({ 
            ...prev, 
            lastError: errorPayload?.message || 'Unknown error',
            isLoading: false 
          }));
          setShowError(true);
          break;
      }
    };

    window.addEventListener('message', handleMessage);
    return () => window.removeEventListener('message', handleMessage);
  }, []);

  // Handle iframe load
  const handleIframeLoad = useCallback(() => {
    setState(prev => ({ 
      ...prev, 
      isLoading: false,
      lastRefresh: Date.now()
    }));
    setShowError(false);
  }, []);

  // Handle iframe error
  const handleIframeError = useCallback(() => {
    setState(prev => ({ 
      ...prev, 
      isLoading: false,
      lastError: 'Failed to load preview'
    }));
    setShowError(true);
  }, []);

  // Refresh preview
  const handleRefresh = useCallback(() => {
    setIframeKey(prev => prev + 1);
    setState(prev => ({ 
      ...prev, 
      isLoading: true,
      refreshCount: prev.refreshCount + 1 
    }));
  }, []);

  // Change viewport
  const handleViewportChange = useCallback((viewport: ViewportMode) => {
    setState(prev => ({ ...prev, viewport }));
    vscodeApi.current?.postMessage({
      type: 'viewport-change',
      payload: { viewport, dimensions: VIEWPORT_CONFIGS[viewport] }
    });
  }, []);

  // Open in external browser
  const handleOpenExternal = useCallback(() => {
    if (iframeUrl) {
      window.open(iframeUrl, '_blank');
    }
  }, [iframeUrl]);

  // Get viewport dimensions
  const getViewportDimensions = (): ViewportDimensions => {
    return VIEWPORT_CONFIGS[state.viewport];
  };

  const dimensions = getViewportDimensions();
  const hasServer = !!state.server;
  const isEmpty = !hasServer && !state.isLoading && !state.lastError;

  // Viewport icons
  const viewportIcons = {
    desktop: Monitor,
    tablet: Tablet,
    mobile: Smartphone
  };

  return (
    <div className={cn(
      "sentinel-preview-panel flex flex-col h-full bg-[#1e1e1e]",
      className
    )}>
      {/* Toolbar */}
      {window.config?.showToolbar !== false && (
        <div className="sentinel-preview-toolbar flex items-center justify-between px-3 py-2 bg-[#252526] border-b border-[#3c3c3c]">
          <div className="flex items-center gap-1">
            {/* Viewport Controls */}
            <div className="flex items-center bg-[#1e1e1e] rounded-md p-0.5">
              {(Object.keys(viewportIcons) as ViewportMode[]).map((mode) => {
                const Icon = viewportIcons[mode];
                return (
                  <button
                    key={mode}
                    onClick={() => handleViewportChange(mode)}
                    className={cn(
                      "p-1.5 rounded transition-all duration-200",
                      state.viewport === mode 
                        ? "bg-[#0e639c] text-white" 
                        : "text-[#cccccc] hover:bg-[#3c3c3c] hover:text-white"
                    )}
                    title={`${mode.charAt(0).toUpperCase() + mode.slice(1)} view`}
                  >
                    <Icon className="w-4 h-4" />
                  </button>
                );
              })}
            </div>

            <div className="w-px h-5 bg-[#3c3c3c] mx-2" />

            {/* URL Display */}
            <div className="flex items-center gap-2 text-sm text-[#cccccc]">
              {hasServer ? (
                <>
                  <CheckCircle2 className="w-3.5 h-3.5 text-green-500" />
                  <span className="font-mono text-xs opacity-80">
                    {iframeUrl}
                  </span>
                </>
              ) : (
                <span className="text-[#808080] italic">No server detected</span>
              )}
            </div>
          </div>

          <div className="flex items-center gap-1">
            {/* Refresh Button */}
            <Button
              variant="ghost"
              size="icon"
              onClick={handleRefresh}
              disabled={!hasServer || state.isLoading}
              className="h-7 w-7 text-[#cccccc] hover:bg-[#3c3c3c] hover:text-white disabled:opacity-40"
              title="Refresh preview"
            >
              <RefreshCw className={cn(
                "w-4 h-4",
                state.isLoading && "animate-spin"
              )} />
            </Button>

            {/* Open External */}
            <Button
              variant="ghost"
              size="icon"
              onClick={handleOpenExternal}
              disabled={!hasServer}
              className="h-7 w-7 text-[#cccccc] hover:bg-[#3c3c3c] hover:text-white disabled:opacity-40"
              title="Open in browser"
            >
              <ExternalLink className="w-4 h-4" />
            </Button>

            {/* Fullscreen Toggle */}
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7 text-[#cccccc] hover:bg-[#3c3c3c] hover:text-white"
              title="Toggle fullscreen"
            >
              <Maximize2 className="w-4 h-4" />
            </Button>
          </div>
        </div>
      )}

      {/* Preview Area */}
      <div 
        className="flex-1 relative overflow-auto bg-[#1e1e1e] flex items-center justify-center"
        onMouseEnter={() => setIsHovering(true)}
        onMouseLeave={() => setIsHovering(false)}
      >
        {/* Empty State */}
        {isEmpty && (
          <div className="text-center p-8">
            <div className="w-16 h-16 mx-auto mb-4 rounded-full bg-[#252526] flex items-center justify-center">
              <Monitor className="w-8 h-8 text-[#6e6e6e]" />
            </div>
            <h3 className="text-[#cccccc] font-medium mb-2">No Preview Available</h3>
            <p className="text-[#808080] text-sm max-w-xs mx-auto">
              Start your development server to see live preview here. Supported: Vite, Next.js, React, Vue, and more.
            </p>
            <div className="mt-4 text-xs text-[#6e6e6e]">
              Scanning ports: 3000, 5173, 8080...
            </div>
          </div>
        )}

        {/* Loading State */}
        {state.isLoading && (
          <div className="absolute inset-0 flex items-center justify-center bg-[#1e1e1e] z-10">
            <div className="flex flex-col items-center gap-3">
              <Loader2 className="w-8 h-8 text-[#0e639c] animate-spin" />
              <span className="text-[#cccccc] text-sm">Loading preview...</span>
            </div>
          </div>
        )}

        {/* Error State */}
        {showError && state.lastError && (
          <div className="absolute inset-0 flex items-center justify-center bg-[#1e1e1e] z-20">
            <div className="text-center p-6 max-w-sm">
              <div className="w-12 h-12 mx-auto mb-3 rounded-full bg-red-900/30 flex items-center justify-center">
                <AlertCircle className="w-6 h-6 text-red-400" />
              </div>
              <h3 className="text-red-400 font-medium mb-2">Preview Error</h3>
              <p className="text-[#808080] text-sm mb-4">{state.lastError}</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => {
                  setShowError(false);
                  handleRefresh();
                }}
                className="border-[#3c3c3c] text-[#cccccc] hover:bg-[#3c3c3c]"
              >
                <RefreshCw className="w-3.5 h-3.5 mr-2" />
                Retry
              </Button>
            </div>
          </div>
        )}

        {/* Iframe Container */}
        {hasServer && (
          <div
            className={cn(
              "relative transition-all duration-300 ease-out bg-white shadow-2xl",
              isHovering && "ring-2 ring-[#0e639c]"
            )}
            style={{
              width: state.viewport === 'desktop' ? '100%' : `${dimensions.width}px`,
              height: state.viewport === 'desktop' ? '100%' : `${dimensions.height}px`,
              maxWidth: '100%',
              maxHeight: '100%'
            }}
          >
            <iframe
              ref={iframeRef}
              key={iframeKey}
              src={iframeUrl}
              className="w-full h-full border-0"
              sandbox="allow-forms allow-modals allow-popups allow-scripts allow-same-origin"
              onLoad={handleIframeLoad}
              onError={handleIframeError}
              title="Live Preview"
            />

            {/* Viewport Label */}
            {state.viewport !== 'desktop' && (
              <div className="absolute -bottom-6 left-1/2 -translate-x-1/2 text-xs text-[#6e6e6e] whitespace-nowrap">
                {dimensions.width} × {dimensions.height}
              </div>
            )}
          </div>
        )}
      </div>

      {/* Status Bar */}
      {hasServer && (
        <div className="flex items-center justify-between px-3 py-1.5 bg-[#252526] border-t border-[#3c3c3c] text-xs">
          <div className="flex items-center gap-3 text-[#808080]">
            <span className="flex items-center gap-1.5">
              <span className="w-2 h-2 rounded-full bg-green-500 animate-pulse" />
              Live
            </span>
            <span>Refreshed {state.refreshCount} times</span>
            {state.lastRefresh > 0 && (
              <span className="text-[#6e6e6e]">
                Last: {new Date(state.lastRefresh).toLocaleTimeString()}
              </span>
            )}
          </div>
          <div className="text-[#6e6e6e]">
            {state.viewport.charAt(0).toUpperCase() + state.viewport.slice(1)} • {dimensions.width}×{dimensions.height}
          </div>
        </div>
      )}
    </div>
  );
};

export default PreviewPanel;
