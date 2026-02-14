import { ChildProcess, spawn } from "child_process";
import { EventEmitter } from "events";

/**
 * Line-delimited JSON transport over stdin/stdout of a child process.
 * Handles spawning, sending requests, and receiving responses/notifications.
 */
export class StdioTransport extends EventEmitter {
  private process: ChildProcess | null = null;
  private buffer: string = "";
  private _connected: boolean = false;

  constructor(
    private command: string,
    private args: string[],
    private cwd: string,
    private envOverrides: NodeJS.ProcessEnv = {},
  ) {
    super();
  }

  get connected(): boolean {
    return this._connected;
  }

  start(): void {
    const commandToRun = `"${this.command}"`; // Quote command for shell safety
    console.log(
      `[MCP Transport] Spawning via shell: ${commandToRun} ${this.args.join(" ")}`,
    );
    console.log(`[MCP Transport] CWD: ${this.cwd}`);

    try {
      this.process = spawn(commandToRun, this.args, {
        cwd: this.cwd,
        shell: true,
        stdio: ["pipe", "pipe", "pipe"],
        env: {
          ...process.env,
          ...this.envOverrides,
        },
      });

      this.process.stdout?.on("data", (chunk: Buffer) => {
        const text = chunk.toString("utf-8");
        // console.log(`[MCP Transport RECV] ${text}`); // Debug extreme
        this.buffer += text;
        this.processBuffer();
      });

      this.process.stderr?.on("data", (chunk: Buffer) => {
        const text = chunk.toString("utf-8").trim();
        if (text) {
          console.error(`[MCP Backend Error] ${text}`);
          this.emit("stderr", text);
        }
      });

      this.process.on("error", (err) => {
        console.error(`[MCP Spawn Error]`, err);
        this._connected = false;
        this.emit("error", err);
      });

      this.process.on("close", (code) => {
        console.log(`[MCP Process Closed] Exit code: ${code}`);
        this._connected = false;
        this.emit("close", code);
      });

      this._connected = true;
      this.emit("connected");
    } catch (e) {
      console.error(`[MCP Fatal Spawn Error]`, e);
      this.emit("error", e);
    }
  }

  send(data: object): void {
    if (!this.process?.stdin?.writable) {
      console.error("[MCP Transport] Cannot send: not connected");
      throw new Error("Transport not connected");
    }
    const json = JSON.stringify(data);
    console.log(`[MCP Transport SEND] ${json}`);
    this.process.stdin.write(json + "\n");
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
    const lines = this.buffer.split("\n");
    this.buffer = lines.pop() || "";

    for (const line of lines) {
      const trimmed = line.trim();
      if (!trimmed) continue;

      if (!trimmed.startsWith("{") && !trimmed.startsWith("[")) {
        this.emit("stderr", trimmed);
        continue;
      }

      console.log(`[MCP Transport RECV] ${trimmed}`);
      try {
        const parsed = JSON.parse(trimmed);
        this.emit("message", parsed);
      } catch (e) {
        console.warn(`[MCP Transport] Failed to parse message: ${trimmed}`, e);
      }
    }
  }
}
