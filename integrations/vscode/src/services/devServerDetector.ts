/**
 * DevServerDetector - Intelligent development server detection engine
 * Scans workspace for running dev servers with zero configuration
 */

import * as vscode from 'vscode';
import * as http from 'http';
import * as https from 'https';
import * as fs from 'fs';
import * as path from 'path';
import { promisify } from 'util';
import {
  DevServer,
  DevServerType,
  DevServerDetectionConfig,
  DEFAULT_DETECTION_CONFIG,
  HealthCheckResponse,
  DetectionResult
} from '../shared/livePreviewTypes';

const readFileAsync = promisify(fs.readFile);
const accessAsync = promisify(fs.access);

export class DevServerDetector {
  private config: DevServerDetectionConfig;
  private cachedResult: DetectionResult | null = null;
  private lastScanTime: number = 0;
  private scanDebounceMs: number = 5000; // Don't scan more often than 5 seconds

  constructor(config: Partial<DevServerDetectionConfig> = {}) {
    this.config = { ...DEFAULT_DETECTION_CONFIG, ...config };
  }

  /**
   * Detect all running development servers in the workspace
   * Uses caching to avoid excessive scanning
   */
  async detectServers(): Promise<DetectionResult> {
    const now = Date.now();
    
    // Return cached result if recent
    if (this.cachedResult && (now - this.lastScanTime) < this.scanDebounceMs) {
      return this.cachedResult;
    }

    const startTime = now;
    const servers: DevServer[] = [];

    // Get workspace folders
    const workspaceFolders = vscode.workspace.workspaceFolders;
    if (!workspaceFolders) {
      return this.createEmptyResult(startTime);
    }

    // Detect server type from config files
    const detectedType = await this.detectServerType(workspaceFolders[0].uri.fsPath);

    // Scan all configured ports
    const scanPromises = this.config.ports.map(port =>
      this.checkPort(port, detectedType)
    );

    const results = await Promise.allSettled(scanPromises);

    results.forEach((result, index) => {
      if (result.status === 'fulfilled' && result.value) {
        servers.push(result.value);
      }
    });

    // Sort by preference (HMR enabled servers first)
    servers.sort((a, b) => (b.hmr ? 1 : 0) - (a.hmr ? 1 : 0));

    const detectionResult: DetectionResult = {
      servers,
      scannedPorts: this.config.ports,
      duration: Date.now() - startTime,
      timestamp: Date.now()
    };

    // Update cache
    this.cachedResult = detectionResult;
    this.lastScanTime = Date.now();

    return detectionResult;
  }

  /**
   * Quick check for most common ports (performance optimization)
   */
  async quickDetect(): Promise<DevServer | null> {
    const priorityPorts = [3000, 5173, 8080, 4000];
    const workspaceFolders = vscode.workspace.workspaceFolders;
    
    if (!workspaceFolders) return null;

    const detectedType = await this.detectServerType(workspaceFolders[0].uri.fsPath);

    for (const port of priorityPorts) {
      const server = await this.checkPort(port, detectedType);
      if (server) return server;
    }

    return null;
  }

  /**
   * Detect server type from configuration files
   */
  private async detectServerType(workspacePath: string): Promise<DevServerType | null> {
    for (const [type, files] of Object.entries(this.config.configFiles)) {
      for (const file of files) {
        const filePath = path.join(workspacePath, file);
        try {
          await accessAsync(filePath, fs.constants.F_OK);
          
          // For package.json, verify it has the right scripts
          if (file === 'package.json') {
            const hasServer = await this.verifyPackageJsonScripts(filePath, type as DevServerType);
            if (hasServer) return type as DevServerType;
          } else {
            return type as DevServerType;
          }
        } catch {
          continue;
        }
      }
    }
    return null;
  }

  /**
   * Verify package.json has appropriate dev scripts
   */
  private async verifyPackageJsonScripts(
    filePath: string,
    type: DevServerType
  ): Promise<boolean> {
    try {
      const content = await readFileAsync(filePath, 'utf-8');
      const pkg = JSON.parse(content);
      const scripts = pkg.scripts || {};

      const scriptPatterns: Record<DevServerType, string[]> = {
        vite: ['dev', 'vite'],
        nextjs: ['dev', 'next dev'],
        nuxt: ['dev', 'nuxt dev'],
        'react-scripts': ['start', 'react-scripts start'],
        'vue-cli': ['serve', 'vue-cli-service serve'],
        angular: ['start', 'ng serve'],
        sveltekit: ['dev', 'vite dev'],
        astro: ['dev', 'astro dev'],
        remix: ['dev', 'remix dev'],
        gatsby: ['develop', 'gatsby develop'],
        parcel: ['start', 'parcel'],
        webpack: ['start', 'webpack serve'],
        custom: ['dev', 'start', 'serve']
      };

      const patterns = scriptPatterns[type] || [];
      return Object.values(scripts).some((script: string) =>
        patterns.some(pattern => script.includes(pattern))
      );
    } catch {
      return false;
    }
  }

  /**
   * Check if a specific port has a running server
   */
  private async checkPort(
    port: number,
    detectedType: DevServerType | null
  ): Promise<DevServer | null> {
    const url = `http://localhost:${port}`;

    try {
      const health = await this.healthCheck(url);
      
      if (health.healthy) {
        // Try to determine server type from response headers
        const serverType = await this.inferServerType(url, detectedType);
        
        return {
          type: serverType,
          port,
          path: '/',
          hmr: this.detectHMR(url, serverType),
          lastSeen: Date.now(),
          healthy: true
        };
      }
    } catch {
      // Port not responding
    }

    return null;
  }

  /**
   * Perform health check on URL
   */
  private healthCheck(url: string): Promise<HealthCheckResponse> {
    return new Promise((resolve) => {
      const startTime = Date.now();
      const timeout = setTimeout(() => {
        resolve({ healthy: false, status: 0, latency: 0, timestamp: Date.now() });
      }, this.config.timeout);

      const protocol = url.startsWith('https') ? https : http;
      
      const request = protocol.get(url, { timeout: this.config.timeout }, (response) => {
        clearTimeout(timeout);
        resolve({
          healthy: response.statusCode === 200 || response.statusCode === 304,
          status: response.statusCode || 0,
          latency: Date.now() - startTime,
          timestamp: Date.now()
        });
      });

      request.on('error', () => {
        clearTimeout(timeout);
        resolve({ healthy: false, status: 0, latency: 0, timestamp: Date.now() });
      });

      request.on('timeout', () => {
        request.destroy();
      });
    });
  }

  /**
   * Infer server type from response headers and behavior
   */
  private async inferServerType(
    url: string,
    detectedType: DevServerType | null
  ): Promise<DevServerType> {
    // Use detected type from config files as primary source
    if (detectedType) return detectedType;

    // Fallback: try to detect from headers (simplified)
    return new Promise((resolve) => {
      const protocol = url.startsWith('https') ? https : http;
      
      const request = protocol.get(url, { timeout: 1000 }, (response) => {
        const server = response.headers['server'] || '';
        const poweredBy = response.headers['x-powered-by'] || '';

        if (server.includes('Vite') || poweredBy.includes('Vite')) {
          resolve('vite');
        } else if (server.includes('Next.js') || poweredBy.includes('Next.js')) {
          resolve('nextjs');
        } else {
          resolve('custom');
        }
      });

      request.on('error', () => resolve('custom'));
      request.setTimeout(1000, () => {
        request.destroy();
        resolve('custom');
      });
    });
  }

  /**
   * Detect if server supports HMR
   */
  private detectHMR(url: string, type: DevServerType): boolean {
    const hmrEnabledTypes: DevServerType[] = [
      'vite', 'nextjs', 'nuxt', 'sveltekit', 'vue-cli', 'react-scripts'
    ];
    return hmrEnabledTypes.includes(type);
  }

  /**
   * Force refresh detection (clear cache)
   */
  async refresh(): Promise<DetectionResult> {
    this.cachedResult = null;
    this.lastScanTime = 0;
    return this.detectServers();
  }

  /**
   * Create empty detection result
   */
  private createEmptyResult(startTime: number): DetectionResult {
    return {
      servers: [],
      scannedPorts: this.config.ports,
      duration: Date.now() - startTime,
      timestamp: Date.now()
    };
  }
}

// Export singleton instance
export const devServerDetector = new DevServerDetector();
