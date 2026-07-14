import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import './index.css'
import './i18n'
import App from './App.tsx'

// macOS keeps native traffic lights over the title bar (titleBarStyle:
// Overlay); mark the root so the header can leave room for them on the left.
if (/Mac|iPhone|iPad|iPod/.test(navigator.platform)) {
  document.documentElement.dataset.os = 'macos'
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
)
