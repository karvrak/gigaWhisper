import React from 'react';
import ReactDOM from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import App from './App';
import { RecordingIndicatorWindow } from './windows/RecordingIndicator';
import { PopupWindow } from './windows/PopupWindow';
import './styles/globals.css';

// Get current window label to render appropriate component
const windowLabel = getCurrentWindow().label;

// For transparent windows, remove background classes
if (windowLabel === 'recording-indicator') {
  document.body.className = '';
  document.body.style.background = 'transparent';
  document.documentElement.style.background = 'transparent';
}

function getWindowComponent() {
  switch (windowLabel) {
    case 'recording-indicator':
      return <RecordingIndicatorWindow />;
    case 'popup':
      return <PopupWindow />;
    default:
      return <App />;
  }
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {getWindowComponent()}
  </React.StrictMode>
);
