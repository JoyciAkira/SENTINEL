import { ChildProcess, spawn } from 'child_process';
import { EventEmitter } from 'events';

/**
 * Line-delimited JSON transport over stdin/stdout of a child process.
 * Handles spawning, sending requests, and receiving responses/notifications.
 */
export class StdioTransport extends EventEmitter {
    private process: ChildProcess | null = null;
    private buffer: string = '';
    private _connected: boolean = false;

    constructor(
        private command: string,
        private args: string[],
        private cwd: string
    ) {
        super();
    }

    get connected(): boolean {
        return this._connected;
    }

    start(): void {
        this.process = spawn(this.command, this.args, {
            cwd: this.cwd,
            shell: false,
            stdio: ['pipe', 'pipe', 'pipe'],
        });

        this.process.stdout?.on('data', (chunk: Buffer) => {
            this.buffer += chunk.toString('utf-8');
            this.processBuffer();
        });

        this.process.stderr?.on('data', (chunk: Buffer) => {
            const text = chunk.toString('utf-8').trim();
            if (text) {
                this.emit('stderr', text);
            }
        });

        this.process.on('error', (err) => {
            this._connected = false;
            this.emit('error', err);
        });

        this.process.on('close', (code) => {
            this._connected = false;
            this.emit('close', code);
        });

        this._connected = true;
        this.emit('connected');
    }

    send(data: object): void {
        if (!this.process?.stdin?.writable) {
            throw new Error('Transport not connected');
        }
        const json = JSON.stringify(data);
        this.process.stdin.write(json + '\n');
    }

    stop(): void {
        if (this.process) {
            this.process.stdin?.end();
            this.process.kill();
            this.process = null;
            this._connected = false;
        }
    }

    private processBuffer(): void {
        const lines = this.buffer.split('\n');
        // Keep the last incomplete line in the buffer
        this.buffer = lines.pop() || '';

        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed) continue;

            try {
                const parsed = JSON.parse(trimmed);
                this.emit('message', parsed);
            } catch {
                // Skip non-JSON lines (e.g. stderr leaking to stdout)
            }
        }
    }
}
