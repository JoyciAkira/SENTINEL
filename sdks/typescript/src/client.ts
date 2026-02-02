import { execFile } from 'child_process';
import { promisify } from 'util';
import { GoalManifold } from './types';

const execFileAsync = promisify(execFile);

export class SentinelError extends Error {
    constructor(message: string) {
        super(message);
        this.name = 'SentinelError';
    }
}

export class SentinelClient {
    private executable: string;
    private workingDir: string;

    constructor(executable: string = 'sentinel', workingDir?: string) {
        this.executable = executable;
        this.workingDir = workingDir || process.cwd();
    }

    private async runCommand(args: string[]): Promise<string> {
        try {
            const { stdout } = await execFileAsync(this.executable, args, {
                cwd: this.workingDir
            });
            return stdout;
        } catch (error: any) {
            if (error.code === 'ENOENT') {
                throw new SentinelError(`Sentinel executable '${this.executable}' not found.`);
            }
            throw new SentinelError(`Sentinel command failed: ${error.stderr || error.message}`);
        }
    }

    async init(description: string): Promise<void> {
        await this.runCommand(['init', description]);
    }

    async status(): Promise<GoalManifold> {
        const output = await this.runCommand(['status', '--json']);
        try {
            const data = JSON.parse(output);
            if (data.manifold) {
                // Adapt goal_dag to goals list
                const goals = data.manifold.goal_dag?.nodes || [];
                return {
                    ...data.manifold,
                    goals
                } as GoalManifold;
            }
            throw new SentinelError('Invalid status response format');
        } catch (e) {
            if (e instanceof SentinelError) throw e;
            throw new SentinelError(`Failed to parse Sentinel output: ${e}`);
        }
    }

    async verify(sandbox: boolean = true): Promise<boolean> {
        const args = ['verify'];
        if (!sandbox) {
            args.push('--sandbox=false');
        }

        try {
            await this.runCommand(args);
            return true;
        } catch {
            return false;
        }
    }

    async decompose(goalId: string): Promise<void> {
        await this.runCommand(['decompose', goalId]);
    }
}
