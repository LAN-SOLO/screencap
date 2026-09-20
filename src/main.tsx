import React from 'react';
import ReactDOM from 'react-dom/client';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import App from './App';
import { RegionPicker } from './components/RegionPicker';
import './styles.css';

// ein Bundle, zwei Fenster: die App und das transparente Auswahl-Overlay
// der Ausschnitt-Aufnahme (Windows, s. capture.rs)
let isRegion = false;
try {
  isRegion = getCurrentWebviewWindow().label === 'region';
} catch {
  /* Browser-Vorschau ohne Tauri */
}
if (isRegion) document.documentElement.dataset.window = 'region';

// Theme synchron vor dem ersten Render setzen (Spiegel der Settings, s. App.tsx)
try {
  const stored = localStorage.getItem('theme');
  document.documentElement.dataset.theme = stored === 'light' ? 'light' : 'dark';
} catch {
  document.documentElement.dataset.theme = 'dark';
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>{isRegion ? <RegionPicker /> : <App />}</React.StrictMode>
);
