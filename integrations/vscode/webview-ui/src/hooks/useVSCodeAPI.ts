import { useMemo } from 'react';

interface VSCodeAPI {
    postMessage(message: unknown): void;
    getState(): unknown;
    setState(state: unknown): void;
}

declare function acquireVsCodeApi(): VSCodeAPI;

let cachedApi: VSCodeAPI | null = null;

export function useVSCodeAPI(): VSCodeAPI {
    return useMemo(() => {
        if (!cachedApi) {
            cachedApi = acquireVsCodeApi();
        }
        return cachedApi;
    }, []);
}
