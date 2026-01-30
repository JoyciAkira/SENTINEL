import * as vscode from 'vscode';
/**
 * Returns the HTML content for the Sentinel Chat webview.
 * In development, loads from Vite dev server.
 * In production, loads the built assets from out/webview/.
 */
export declare function getWebviewContent(webview: vscode.Webview, extensionUri: vscode.Uri): string;
