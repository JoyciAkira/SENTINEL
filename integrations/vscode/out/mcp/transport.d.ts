import { EventEmitter } from 'events';
/**
 * Line-delimited JSON transport over stdin/stdout of a child process.
 * Handles spawning, sending requests, and receiving responses/notifications.
 */
export declare class StdioTransport extends EventEmitter {
    private command;
    private args;
    private cwd;
    private process;
    private buffer;
    private _connected;
    constructor(command: string, args: string[], cwd: string);
    get connected(): boolean;
    start(): void;
    send(data: object): void;
    stop(): void;
    private processBuffer;
}
