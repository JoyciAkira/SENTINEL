import { SentinelClient, GoalManifold, Goal } from '@sentinel/sdk';
import * as vscode from 'vscode';
import * as path from 'path';
import * as fs from 'fs';

export class SentinelService {
    private client: SentinelClient | undefined;
    private outputChannel: vscode.OutputChannel;

    constructor() {
        this.outputChannel = vscode.window.createOutputChannel("Sentinel Core");
    }

    public async initialize(workspaceRoot: string): Promise<boolean> {
        try {
            const binaryPath = await this.findBinaryPath();
            this.log(`Initializing Sentinel Client with binary: ${binaryPath}`);
            this.log(`Workspace: ${workspaceRoot}`);

            try {
                this.client = new SentinelClient(binaryPath, workspaceRoot);
            } catch (clientError: any) {
                this.client = undefined;
                this.log(`⚠️ SDK client bootstrap skipped: ${clientError?.message || String(clientError)}`);
                this.log(`⚠️ Continuing in MCP-first mode.`);
                return true;
            }

            // Verify connection with best-effort probing.
            // MCP is the authoritative transport; SDK probe should never block activation.
            try {
                const manifold = await this.client.status();
                const rootIntent = this.extractRootIntentDescription(manifold);
                this.log(`✅ Connected. Root Intent: ${rootIntent}`);
            } catch (probeError: any) {
                this.log(`⚠️ SDK status probe skipped: ${probeError?.message || String(probeError)}`);
                this.log(`⚠️ Continuing in MCP-first mode.`);
            }
            return true;
        } catch (error: any) {
            this.log(`❌ Initialization Failed: ${error.message}`);
            this.log(`⚠️ Continuing in MCP-first mode.`);
            // Do not block extension activation on SDK probe errors.
            return true;
        }
    }

    public async getStatus(): Promise<GoalManifold | null> {
        if (!this.client) return null;
        try {
            return await this.client.status();
        } catch (e: any) {
            this.log(`Error fetching status: ${e.message}`);
            return null;
        }
    }

    public async decompose(goalId: string): Promise<void> {
        if (!this.client) return;
        this.log(`Decomposing goal ${goalId}...`);
        await this.client.decompose(goalId);
    }

    private log(msg: string) {
        this.outputChannel.appendLine(`[${new Date().toLocaleTimeString()}] ${msg}`);
    }

    private async findBinaryPath(): Promise<string> {
        // 1. Check configuration
        const config = vscode.workspace.getConfiguration('sentinel');
        const configPath =
            config.get<string>('path') ||
            config.get<string>('binaryPath');
        if (configPath && fs.existsSync(configPath)) return configPath;

        // 2. Check local dev build (target/release) - Good for dogfooding
        // Assuming we are in integrations/vscode/src/services
        // We need to go up to root: ../../../../target/release/sentinel-cli
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (workspaceFolders) {
            const root = workspaceFolders[0].uri.fsPath;
            // Check potential standard paths
            const devPath = path.join(root, 'target', 'release', 'sentinel-cli');
            if (fs.existsSync(devPath)) return devPath;
            
            const devPathShort = path.join(root, 'target', 'release', 'sentinel');
            if (fs.existsSync(devPathShort)) return devPathShort;
        }

        // 3. Check standard user paths (robust discovery)
        const home = process.env.HOME || process.env.USERPROFILE;
        if (home) {
            const locations = [
                path.join(home, ".local", "bin", "sentinel"),
                path.join(home, ".cargo", "bin", "sentinel"),
                path.join("/usr", "local", "bin", "sentinel")
            ];
            
            for (const loc of locations) {
                if (fs.existsSync(loc)) {
                    this.log(`Found binary in standard path: ${loc}`);
                    return loc;
                }
            }
        }

        // 4. Fallback to PATH
        return 'sentinel';
    }

    private extractRootIntentDescription(manifold: any): string {
        const description = manifold?.root_intent?.description;
        if (typeof description === 'string' && description.trim().length > 0) {
            return description;
        }
        return 'N/A';
    }
}

export const sentinelService = new SentinelService();
