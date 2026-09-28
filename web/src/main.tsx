import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import './styles/theme.css';
import App from './App';
import { loadEngine } from './engine/engine';

try {
  document.documentElement.dataset.theme = localStorage.getItem('protoforge.theme') || 'dark';
} catch {
  document.documentElement.dataset.theme = 'dark';
}
loadEngine();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
