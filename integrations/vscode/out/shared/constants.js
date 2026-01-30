"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.RECONNECT_MAX_MS = exports.RECONNECT_BASE_MS = exports.MCP_TOOL_GET_ENFORCEMENT_RULES = exports.MCP_TOOL_GET_COGNITIVE_MAP = exports.MCP_TOOL_RECORD_HANDOVER = exports.MCP_TOOL_PROPOSE_STRATEGY = exports.MCP_TOOL_SAFE_WRITE = exports.MCP_TOOL_GET_ALIGNMENT = exports.MCP_TOOL_VALIDATE_ACTION = exports.POLL_INTERVAL_MS = exports.ALIGNMENT_CRITICAL = exports.ALIGNMENT_DEVIATION = exports.ALIGNMENT_CONCERNING = exports.ALIGNMENT_ACCEPTABLE = exports.ALIGNMENT_GOOD = exports.ALIGNMENT_EXCELLENT = exports.CONFIG_DEFAULT_PATH = exports.CONFIG_SENTINEL_PATH = exports.VIEW_NETWORK = exports.VIEW_SECURITY = exports.VIEW_AGENTS = exports.VIEW_GOALS = exports.VIEW_ALIGNMENT = exports.VIEW_CHAT = exports.CMD_SHOW_ALIGNMENT = exports.CMD_VALIDATE_ACTION = exports.CMD_REFRESH_GOALS = exports.CMD_OPEN_CHAT = void 0;
// Command IDs
exports.CMD_OPEN_CHAT = 'sentinel.openChat';
exports.CMD_REFRESH_GOALS = 'sentinel.refreshGoals';
exports.CMD_VALIDATE_ACTION = 'sentinel.validateAction';
exports.CMD_SHOW_ALIGNMENT = 'sentinel.showAlignment';
// View IDs
exports.VIEW_CHAT = 'sentinel-chat';
exports.VIEW_ALIGNMENT = 'sentinel-alignment';
exports.VIEW_GOALS = 'sentinel-goals';
exports.VIEW_AGENTS = 'sentinel-agents';
exports.VIEW_SECURITY = 'sentinel-security';
exports.VIEW_NETWORK = 'sentinel-network';
// Configuration keys
exports.CONFIG_SENTINEL_PATH = 'sentinel.path';
exports.CONFIG_DEFAULT_PATH = 'sentinel';
// Alignment thresholds
exports.ALIGNMENT_EXCELLENT = 90;
exports.ALIGNMENT_GOOD = 75;
exports.ALIGNMENT_ACCEPTABLE = 60;
exports.ALIGNMENT_CONCERNING = 40;
exports.ALIGNMENT_DEVIATION = 30;
exports.ALIGNMENT_CRITICAL = 15;
// Polling
exports.POLL_INTERVAL_MS = 60000; // 60 seconds
// MCP Tool names
exports.MCP_TOOL_VALIDATE_ACTION = 'validate_action';
exports.MCP_TOOL_GET_ALIGNMENT = 'get_alignment';
exports.MCP_TOOL_SAFE_WRITE = 'safe_write';
exports.MCP_TOOL_PROPOSE_STRATEGY = 'propose_strategy';
exports.MCP_TOOL_RECORD_HANDOVER = 'record_handover';
exports.MCP_TOOL_GET_COGNITIVE_MAP = 'get_cognitive_map';
exports.MCP_TOOL_GET_ENFORCEMENT_RULES = 'get_enforcement_rules';
// Reconnection
exports.RECONNECT_BASE_MS = 1000;
exports.RECONNECT_MAX_MS = 30000;
//# sourceMappingURL=constants.js.map