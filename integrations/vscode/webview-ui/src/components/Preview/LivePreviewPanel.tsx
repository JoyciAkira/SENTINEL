/**
 * LivePreviewPanel — Preview live del dev server.
 *
 * Si connette al canale MCP per ricevere l'URL del dev server rilevato
 * dall'estensione (`livePreviewUrl`). Supporta:
 * - Viewport switching (desktop / tablet / mobile)
 * - Refresh iframe
 * - URL manuale fallback
 * - Hard-reload via postMessage verso l'estensione (`refreshPreview`)
 */
import React, { useCallback, useEffect, useRef, useState } from "react";
import {
  Monitor,
  Smartphone,
  Tablet,
  RefreshCw,
  ExternalLink,
  Link,
  ServerCrash,
  Loader2,
  X,
} from "lucide-react";
import { cn } from "@/lib/utils";
import { Button } from "../ui/button";
import { useVSCodeAPI } from "../../hooks/useVSCodeAPI";

// ─── viewport presets ─────────────────────────────────────────────────────────

type Viewport = "desktop" | "tablet" | "mobile";

const VIEWPORTS: Record<Viewport, { width: string; label: string; icon: React.ReactNode }> = {
  desktop: { width: "100%",  label: "Desktop", icon: <Monitor    className="size-3.5" /> },
  tablet:  { width: "768px", label: "Tablet",  icon: <Tablet     className="size-3.5" /> },
  mobile:  { width: "390px", label: "Mobile",  icon: <Smartphone className="size-3.5" /> },
};

// ─── component ────────────────────────────────────────────────────────────────

interface LivePreviewPanelProps {
  /** Initial URL, e.g. injected from extension state */
  initialUrl?: string;
}

export const LivePreviewPanel: React.FC<LivePreviewPanelProps> = ({ initialUrl }) => {
  const vscode           = useVSCodeAPI();
  const iframeRef        = useRef<HTMLIFrameElement>(null);

  const [url, setUrl]               = useState<string>(initialUrl ?? "");
  const [inputUrl, setInputUrl]     = useState<string>(initialUrl ?? "");
  const [viewport, setViewport]     = useState<Viewport>("desktop");
  const [loading, setLoading]       = useState(false);
  const [loadError, setLoadError]   = useState(false);
  const [key, setKey]               = useState(0); // increment to force iframe re-mount

  // Listen for `livePreviewUrl` from the extension host
  useEffect(() => {
    const handler = (event: MessageEvent) => {
      if (!event.data || typeof event.data !== "object") return;
      if (event.data.type === "livePreviewUrl" && typeof event.data.url === "string") {
        const receivedUrl = event.data.url;
        setUrl(receivedUrl);
        setInputUrl(receivedUrl);
        setLoadError(false);
      }
    };
    window.addEventListener("message", handler);
    // Request URL on mount
    vscode.postMessage({ type: "getPreviewUrl" });
    return () => window.removeEventListener("message", handler);
  }, [vscode]);

  const navigate = useCallback((target: string) => {
    const trimmed = target.trim();
    if (!trimmed) return;
    // Normalise: add http:// if no protocol
    const finalUrl = /^https?:\/\//.test(trimmed) ? trimmed : `http://${trimmed}`;
    setUrl(finalUrl);
    setLoadError(false);
    setLoading(true);
    setKey((k) => k + 1); // force iframe re-mount
  }, []);

  const refresh = useCallback(() => {
    if (!url) return;
    setLoadError(false);
    setLoading(true);
    // Try to refresh without full re-mount first
    try {
      iframeRef.current?.contentWindow?.location.reload();
      setTimeout(() => setLoading(false), 500);
    } catch {
      // Cross-origin: fall back to re-mount
      setKey((k) => k + 1);
    }
    vscode.postMessage({ type: "refreshPreview", url });
  }, [url, vscode]);

  const openExternal = useCallback(() => {
    if (url) vscode.postMessage({ type: "openExternal", url });
  }, [url, vscode]);

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* ── Toolbar ── */}
      <div
        className="shrink-0 flex items-center gap-2 px-3 py-2 border-b border-border"
        style={{ background: "color-mix(in oklab, var(--sentinel-surface) 97%, transparent)" }}
      >
        {/* Viewport switcher */}
        <div className="flex items-center border border-border rounded-lg overflow-hidden shrink-0">
          {(Object.entries(VIEWPORTS) as [Viewport, typeof VIEWPORTS[Viewport]][]).map(
            ([key, vp]) => (
              <button
                key={key}
                type="button"
                title={vp.label}
                onClick={() => setViewport(key)}
                className={cn(
                  "flex items-center justify-center w-8 h-7 transition-colors",
                  viewport === key
                    ? "bg-primary/20 text-primary"
                    : "text-muted-foreground hover:text-foreground hover:bg-card/60",
                )}
              >
                {vp.icon}
              </button>
            )
          )}
        </div>

        {/* URL input */}
        <form
          className="flex flex-1 items-center gap-1 border border-border rounded-lg overflow-hidden bg-card/50 min-w-0"
          onSubmit={(e) => { e.preventDefault(); navigate(inputUrl); }}
        >
          <Link className="size-3.5 ml-2 text-muted-foreground shrink-0" />
          <input
            type="text"
            value={inputUrl}
            onChange={(e) => setInputUrl(e.target.value)}
            placeholder="http://localhost:3000"
            className="flex-1 bg-transparent text-xs py-1.5 px-2 focus:outline-none text-foreground placeholder:text-muted-foreground min-w-0"
          />
          {inputUrl && (
            <button
              type="button"
              onClick={() => { setInputUrl(""); setUrl(""); }}
              className="text-muted-foreground hover:text-foreground px-1.5 shrink-0"
            >
              <X className="size-3" />
            </button>
          )}
        </form>

        {/* Actions */}
        <Button
          size="icon"
          variant="ghost"
          className="size-7 shrink-0"
          title="Refresh"
          onClick={refresh}
          disabled={!url}
        >
          <RefreshCw className={cn("size-3.5", loading && "animate-spin")} />
        </Button>
        <Button
          size="icon"
          variant="ghost"
          className="size-7 shrink-0"
          title="Open in browser"
          onClick={openExternal}
          disabled={!url}
        >
          <ExternalLink className="size-3.5" />
        </Button>
      </div>

      {/* ── Canvas ── */}
      <div
        className="flex-1 min-h-0 flex items-center justify-center overflow-auto"
        style={{ background: "#1e1e1e" }}
      >
        {!url ? (
          /* Empty state */
          <div className="flex flex-col items-center gap-4 text-muted-foreground py-16 px-8 text-center">
            <Monitor className="size-14 opacity-20" />
            <div>
              <p className="text-sm font-medium text-foreground/80">No dev server detected</p>
              <p className="text-xs mt-1">
                Start your dev server (<span className="font-mono">npm run dev</span>) and
                Sentinel will auto-detect the URL, or enter it manually above.
              </p>
            </div>
            <Button
              size="sm"
              variant="outline"
              onClick={() => vscode.postMessage({ type: "getPreviewUrl" })}
              className="gap-2"
            >
              <RefreshCw className="size-3.5" /> Scan for dev servers
            </Button>
          </div>
        ) : loadError ? (
          /* Error state */
          <div className="flex flex-col items-center gap-4 text-muted-foreground py-16 px-8 text-center">
            <ServerCrash className="size-14 opacity-20" />
            <div>
              <p className="text-sm font-medium text-foreground/80">Preview failed to load</p>
              <p className="text-xs mt-1 font-mono">{url}</p>
              <p className="text-xs mt-1">
                The server may be down or blocking iframe embedding (X-Frame-Options).
              </p>
            </div>
            <Button size="sm" variant="outline" onClick={openExternal} className="gap-2">
              <ExternalLink className="size-3.5" /> Open in browser instead
            </Button>
          </div>
        ) : (
          /* Iframe viewport */
          <div
            style={{
              width:       VIEWPORTS[viewport].width,
              height:      "100%",
              position:    "relative",
              transition:  "width 300ms ease",
              background:  "white",
              boxShadow:   "0 0 0 1px rgba(255,255,255,0.08), 0 12px 40px rgba(0,0,0,0.5)",
              borderRadius: viewport !== "desktop" ? 12 : 0,
              overflow:    "hidden",
            }}
          >
            {loading && (
              <div
                style={{
                  position:       "absolute",
                  inset:          0,
                  display:        "flex",
                  alignItems:     "center",
                  justifyContent: "center",
                  background:     "rgba(0,0,0,0.4)",
                  zIndex:         10,
                }}
              >
                <Loader2 className="size-8 text-primary animate-spin" />
              </div>
            )}
            <iframe
              key={key}
              ref={iframeRef}
              src={url}
              title="Live Preview"
              onLoad={() => setLoading(false)}
              onError={() => { setLoading(false); setLoadError(true); }}
              style={{
                width:  "100%",
                height: "100%",
                border: "none",
                display: "block",
              }}
              allow="cross-origin-isolated"
              sandbox="allow-scripts allow-same-origin allow-forms allow-popups allow-modals"
            />
          </div>
        )}
      </div>

      {/* ── Status bar ── */}
      {url && !loadError && (
        <div
          className="shrink-0 flex items-center justify-between px-3 py-1 border-t border-border text-[10px] text-muted-foreground"
          style={{ background: "color-mix(in oklab, var(--sentinel-surface) 95%, transparent)" }}
        >
          <span className="font-mono truncate">{url}</span>
          <span className="shrink-0 ml-2">{VIEWPORTS[viewport].label}</span>
        </div>
      )}
    </div>
  );
};

export default LivePreviewPanel;
