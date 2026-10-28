import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';
import { diagnosticValue, installDiagnostics } from './performance';

if (import.meta.env.MODE === 'desktop-test') {
  await import('@wdio/tauri-plugin');
  installDiagnostics();
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    {import.meta.env.MODE === 'desktop-test' ? (
      <React.Profiler
        id="diagnostic"
        onRender={(_id, _phase, actualDuration) =>
          diagnosticValue('react_render_ms', actualDuration)
        }
      >
        <App />
      </React.Profiler>
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
