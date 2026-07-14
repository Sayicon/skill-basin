import { useEffect, useState } from 'react'
import { Minus, Square, Copy, X } from 'lucide-react'
import type { TFunction } from 'i18next'

// Present only inside the Tauri webview; in a plain browser (vite dev, the
// web build) there is no OS window to drive, so the controls hide themselves.
const IN_TAURI = typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

type WindowControlsProps = { t: TFunction }

const WindowControls = ({ t }: WindowControlsProps) => {
  const [maximized, setMaximized] = useState(false)

  useEffect(() => {
    if (!IN_TAURI) return
    let unlisten: (() => void) | undefined
    let alive = true
    void (async () => {
      const { getCurrentWindow } = await import('@tauri-apps/api/window')
      const win = getCurrentWindow()
      const sync = async () => {
        try {
          const m = await win.isMaximized()
          if (alive) setMaximized(m)
        } catch {
          /* window gone */
        }
      }
      await sync()
      // The maximize/restore state also changes via OS gestures (snap,
      // double-click, keyboard), so track the window, not just our buttons.
      unlisten = await win.onResized(() => {
        void sync()
      })
    })()
    return () => {
      alive = false
      unlisten?.()
    }
  }, [])

  if (!IN_TAURI) return null

  const withWindow = (fn: (win: import('@tauri-apps/api/window').Window) => void) => () => {
    void import('@tauri-apps/api/window').then(({ getCurrentWindow }) => fn(getCurrentWindow()))
  }

  return (
    <div className="window-controls">
      <button
        type="button"
        className="win-ctl"
        aria-label={t('window.minimize')}
        title={t('window.minimize')}
        onClick={withWindow((w) => void w.minimize())}
      >
        <Minus size={16} />
      </button>
      <button
        type="button"
        className="win-ctl"
        aria-label={maximized ? t('window.restore') : t('window.maximize')}
        title={maximized ? t('window.restore') : t('window.maximize')}
        onClick={withWindow((w) => void w.toggleMaximize())}
      >
        {maximized ? <Copy size={13} /> : <Square size={13} />}
      </button>
      <button
        type="button"
        className="win-ctl win-ctl-close"
        aria-label={t('window.close')}
        title={t('window.close')}
        onClick={withWindow((w) => void w.close())}
      >
        <X size={17} />
      </button>
    </div>
  )
}

export default WindowControls
