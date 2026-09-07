import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';

// Theme synchron vor dem ersten Render setzen (Spiegel der Settings, s. App.tsx)
try {
  const stored = localStorage.getItem('theme');
  document.documentElement.dataset.theme = stored === 'light' ? 'light' : 'dark';
} catch {
  document.documentElement.dataset.theme = 'dark';
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
