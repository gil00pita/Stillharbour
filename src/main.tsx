import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';
import './styles.css';
import { diagnosticValue, installDiagnostics } from './performance';

let GeneratedRenameLab: React.ComponentType | null = null;
if (import.meta.env.MODE === 'desktop-test') {
  await import('@wdio/tauri-plugin');
  GeneratedRenameLab = (await import('./GeneratedRenameLab')).GeneratedRenameLab;
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
        {GeneratedRenameLab && <GeneratedRenameLab />}
      </React.Profiler>
    ) : (
      <App />
    )}
  </React.StrictMode>,
);
