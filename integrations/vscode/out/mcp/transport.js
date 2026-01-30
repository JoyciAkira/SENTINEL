"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.StdioTransport = void 0;
const child_process_1 = require("child_process");
const events_1 = require("events");
/**
 * Line-delimited JSON transport over stdin/stdout of a child process.
 * Handles spawning, sending requests, and receiving responses/notifications.
 */
class StdioTransport extends events_1.EventEmitter {
    constructor(command, args, cwd) {
        super();
        this.command = command;
        this.args = args;
        this.cwd = cwd;
        this.process = null;
        this.buffer = '';
        this._connected = false;
    }
    get connected() {
        return this._connected;
    }
    start() {
        this.process = (0, child_process_1.spawn)(this.command, this.args, {
            cwd: this.cwd,
            shell: false,
            stdio: ['pipe', 'pipe', 'pipe'],
        });
        this.process.stdout?.on('data', (chunk) => {
            this.buffer += chunk.toString('utf-8');
            this.processBuffer();
        });
        this.process.stderr?.on('data', (chunk) => {
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
    send(data) {
        if (!this.process?.stdin?.writable) {
            throw new Error('Transport not connected');
        }
        const json = JSON.stringify(data);
        this.process.stdin.write(json + '\n');
    }
    stop() {
        if (this.process) {
            this.process.stdin?.end();
            this.process.kill();
            this.process = null;
            this._connected = false;
        }
    }
    processBuffer() {
        const lines = this.buffer.split('\n');
        // Keep the last incomplete line in the buffer
        this.buffer = lines.pop() || '';
        for (const line of lines) {
            const trimmed = line.trim();
            if (!trimmed)
                continue;
            try {
                const parsed = JSON.parse(trimmed);
                this.emit('message', parsed);
            }
            catch {
                // Skip non-JSON lines (e.g. stderr leaking to stdout)
            }
        }
    }
}
exports.StdioTransport = StdioTransport;
//# sourceMappingURL=transport.js.map