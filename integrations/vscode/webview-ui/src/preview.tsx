/**
 * Preview Panel Entry Point
 * Standalone entry for the preview webview
 */

import React from 'react';
import ReactDOM from 'react-dom/client';
import PreviewPanel from './components/Preview/PreviewPanel';
import './styles/preview.css';

// Initialize React root
const container = document.getElementById('root');
if (container) {
  const root = ReactDOM.createRoot(container);
  root.render(
    <React.StrictMode>
      <PreviewPanel />
    </React.StrictMode>
  );
}
