// Command IDs
export const CMD_OPEN_CHAT = 'sentinel.openChat';
export const CMD_REFRESH_GOALS = 'sentinel.refreshGoals';
export const CMD_VALIDATE_ACTION = 'sentinel.validateAction';
export const CMD_SHOW_ALIGNMENT = 'sentinel.showAlignment';

// View IDs
export const VIEW_CHAT = 'sentinel-chat';
export const VIEW_ALIGNMENT = 'sentinel-alignment';
export const VIEW_GOALS = 'sentinel-goals';
export const VIEW_AGENTS = 'sentinel-agents';
export const VIEW_SECURITY = 'sentinel-security';
export const VIEW_NETWORK = 'sentinel-network';

// Configuration keys
export const CONFIG_SENTINEL_PATH = 'sentinel.path';
export const CONFIG_DEFAULT_PATH = 'sentinel';

// Alignment thresholds
export const ALIGNMENT_EXCELLENT = 90;
export const ALIGNMENT_GOOD = 75;
export const ALIGNMENT_ACCEPTABLE = 60;
export const ALIGNMENT_CONCERNING = 40;
export const ALIGNMENT_DEVIATION = 30;
export const ALIGNMENT_CRITICAL = 15;

// Polling
export const POLL_INTERVAL_MS = 60_000; // 60 seconds

// MCP Tool names
export const MCP_TOOL_VALIDATE_ACTION = 'validate_action';
export const MCP_TOOL_GET_ALIGNMENT = 'get_alignment';
export const MCP_TOOL_SAFE_WRITE = 'safe_write';
export const MCP_TOOL_PROPOSE_STRATEGY = 'propose_strategy';
export const MCP_TOOL_RECORD_HANDOVER = 'record_handover';
export const MCP_TOOL_GET_COGNITIVE_MAP = 'get_cognitive_map';
export const MCP_TOOL_GET_ENFORCEMENT_RULES = 'get_enforcement_rules';

// Reconnection
export const RECONNECT_BASE_MS = 1000;
export const RECONNECT_MAX_MS = 30_000;
