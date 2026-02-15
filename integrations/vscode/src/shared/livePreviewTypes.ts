/**
 * Sentinel Live Preview - Type Definitions
 * End-to-end type system for real-time development preview
 */

/**
 * Represents a detected development server
 */
export interface DevServer {
  /** Server type/framework detected */
  type: DevServerType;
  /** Localhost port */
  port: number;
  /** Base path (e.g., '/' or '/app') */
  path: string;
  /** Whether the server supports Hot Module Replacement */
  hmr: boolean;
  /** Server process ID (if managed by Sentinel) */
  pid?: number;
  /** Last detected timestamp */
  lastSeen: number;
  /** Health check status */
  healthy: boolean;
}

/**
 * Supported development server types
 */
export type DevServerType =
  | 'vite'
  | 'nextjs'
  | 'nuxt'
  | 'react-scripts'
  | 'vue-cli'
  | 'angular'
  | 'sveltekit'
  | 'astro'
  | 'remix'
  | 'gatsby'
  | 'parcel'
  | 'webpack'
  | 'custom';

/**
 * Configuration for development server detection
 */
export interface DevServerDetectionConfig {
  /** Ports to scan */
  ports: number[];
  /** Timeout for health check (ms) */
  timeout: number;
  /** Retry attempts */
  retries: number;
  /** File patterns to identify server type */
  configFiles: Record<DevServerType, string[]>;
}

/**
 * Default detection configuration
 */
export const DEFAULT_DETECTION_CONFIG: DevServerDetectionConfig = {
  ports: [3000, 3001, 5173, 5174, 8080, 8081, 4200, 5000, 8000, 9000, 1234, 4000],
  timeout: 2000,
  retries: 2,
  configFiles: {
    vite: ['vite.config.ts', 'vite.config.js', 'vite.config.mjs'],
    nextjs: ['next.config.js', 'next.config.ts', 'next.config.mjs'],
    nuxt: ['nuxt.config.ts', 'nuxt.config.js'],
    'react-scripts': ['package.json'],
    'vue-cli': ['vue.config.js'],
    angular: ['angular.json'],
    sveltekit: ['svelte.config.js'],
    astro: ['astro.config.mjs', 'astro.config.ts'],
    remix: ['remix.config.js'],
    gatsby: ['gatsby-config.js'],
    parcel: ['.parcelrc', 'package.json'],
    webpack: ['webpack.config.js'],
    custom: []
  }
};

/**
 * Viewport mode for preview
 */
export type ViewportMode = 'desktop' | 'mobile' | 'tablet';

/**
 * Viewport dimensions
 */
export interface ViewportDimensions {
  mode: ViewportMode;
  width: number;
  height: number;
}

/**
 * Default viewport configurations
 */
export const VIEWPORT_CONFIGS: Record<ViewportMode, ViewportDimensions> = {
  desktop: { mode: 'desktop', width: 1920, height: 1080 },
  tablet: { mode: 'tablet', width: 768, height: 1024 },
  mobile: { mode: 'mobile', width: 375, height: 812 }
};

/**
 * Message types for webview communication
 */
export type PreviewMessageType =
  | 'init'
  | 'url-change'
  | 'viewport-change'
  | 'refresh'
  | 'ready'
  | 'error'
  | 'health-check'
  | 'file-changed'
  | 'sync-request';

/**
 * Message structure for webview communication
 */
export interface PreviewMessage {
  type: PreviewMessageType;
  payload?: unknown;
}

/**
 * Specific message payloads
 */
export interface InitMessage extends PreviewMessage {
  type: 'init';
  payload: {
    url: string;
    viewport: ViewportMode;
    title: string;
  };
}

export interface UrlChangeMessage extends PreviewMessage {
  type: 'url-change';
  payload: {
    url: string;
  };
}

export interface ViewportChangeMessage extends PreviewMessage {
  type: 'viewport-change';
  payload: {
    viewport: ViewportMode;
    dimensions: ViewportDimensions;
  };
}

export interface FileChangedMessage extends PreviewMessage {
  type: 'file-changed';
  payload: {
    filePath: string;
    changeType: 'created' | 'modified' | 'deleted';
  };
}

/**
 * Preview panel state
 */
export interface PreviewPanelState {
  server: DevServer | null;
  viewport: ViewportMode;
  isLoading: boolean;
  lastError: string | null;
  refreshCount: number;
  lastRefresh: number;
}

/**
 * Configuration for Live Preview feature
 */
export interface LivePreviewConfig {
  /** Auto-start preview when server detected */
  autoStart: boolean;
  /** Default viewport mode */
  defaultViewport: ViewportMode;
  /** Refresh delay after file change (ms) */
  refreshDelay: number;
  /** Show toolbar */
  showToolbar: boolean;
  /** Enable auto-sync */
  autoSync: boolean;
}

/**
 * Default Live Preview configuration
 */
export const DEFAULT_PREVIEW_CONFIG: LivePreviewConfig = {
  autoStart: true,
  defaultViewport: 'desktop',
  refreshDelay: 300,
  showToolbar: true,
  autoSync: true
};

/**
 * Health check response
 */
export interface HealthCheckResponse {
  healthy: boolean;
  status: number;
  latency: number;
  timestamp: number;
}

/**
 * File change event for triggering preview refresh
 */
export interface FileChangeEvent {
  uri: string;
  type: 'created' | 'modified' | 'deleted';
  timestamp: number;
}

/**
 * Preview action types
 */
export type PreviewAction =
  | { type: 'detect-server' }
  | { type: 'start-preview'; server: DevServer }
  | { type: 'stop-preview' }
  | { type: 'change-viewport'; viewport: ViewportMode }
  | { type: 'refresh' }
  | { type: 'handle-error'; error: string }
  | { type: 'file-changed'; event: FileChangeEvent };

/**
 * Detection result
 */
export interface DetectionResult {
  servers: DevServer[];
  scannedPorts: number[];
  duration: number;
  timestamp: number;
}
