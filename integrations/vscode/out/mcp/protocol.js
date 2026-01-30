"use strict";
// JSON-RPC 2.0 protocol types for MCP
Object.defineProperty(exports, "__esModule", { value: true });
exports.isNotification = isNotification;
exports.isResponse = isResponse;
// Helper to check if a message is a notification (no id field)
function isNotification(msg) {
    const obj = msg;
    return obj.jsonrpc === '2.0' && typeof obj.method === 'string' && !('id' in obj);
}
// Helper to check if a message is a response (has id field)
function isResponse(msg) {
    const obj = msg;
    return obj.jsonrpc === '2.0' && 'id' in obj;
}
//# sourceMappingURL=protocol.js.map