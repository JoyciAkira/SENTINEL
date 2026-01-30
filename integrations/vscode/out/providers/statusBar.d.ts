import { MCPClient } from '../mcp/client';
import type { AlignmentReport } from '../shared/types';
export declare class SentinelStatusBar {
    private client;
    private item;
    constructor(client: MCPClient);
    update(report: AlignmentReport): void;
    showDisconnected(): void;
    dispose(): void;
}
