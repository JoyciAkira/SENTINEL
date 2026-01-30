export interface JsonRpcRequest {
    jsonrpc: '2.0';
    method: string;
    params?: Record<string, unknown>;
    id: number;
}
export interface JsonRpcResponse {
    jsonrpc: '2.0';
    result?: unknown;
    error?: JsonRpcError;
    id: number | null;
}
export interface JsonRpcNotification {
    jsonrpc: '2.0';
    method: string;
    params?: Record<string, unknown>;
}
export interface JsonRpcError {
    code: number;
    message: string;
    data?: unknown;
}
export interface McpInitializeResult {
    protocolVersion: string;
    capabilities: {
        tools?: {
            listChanged?: boolean;
        };
    };
    serverInfo: {
        name: string;
        version: string;
    };
}
export interface McpToolInfo {
    name: string;
    description: string;
    inputSchema: {
        type: string;
        properties?: Record<string, unknown>;
        required?: string[];
    };
}
export interface McpToolsListResult {
    tools: McpToolInfo[];
}
export interface McpToolCallParams {
    name: string;
    arguments?: Record<string, unknown>;
}
export interface McpToolCallResult {
    content: Array<{
        type: 'text';
        text: string;
    }>;
    isError?: boolean;
}
export declare function isNotification(msg: unknown): msg is JsonRpcNotification;
export declare function isResponse(msg: unknown): msg is JsonRpcResponse;
