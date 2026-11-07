import React from 'react';
import ReactDOM from 'react-dom/client';
import App from '@/components/App';
import '@fontsource-variable/inter/index.css';
import { Provider } from './components/ui/Provider';
import { diagnosticValue, installDiagnostics } from './performance';

let GeneratedRenameLab: React.ComponentType | null = null;
if (import.meta.env.MODE === 'desktop-test') {
  await import('@wdio/tauri-plugin');
  GeneratedRenameLab = (await import('@/components/GeneratedRenameLab')).GeneratedRenameLab;
  installDiagnostics();
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <Provider>
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
    </Provider>
  </React.StrictMode>,
);
