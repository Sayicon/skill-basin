import { memo } from 'react'
import { Layers, Search, Server, Settings, SlidersHorizontal } from 'lucide-react'
import type { TFunction } from 'i18next'
import logoIcon from '../../assets/logo-icon.png'
import WindowControls from './WindowControls'

type HeaderProps = {
  language: string
  loading: boolean
  activeView: 'myskills' | 'explore' | 'detail' | 'settings' | 'manage' | 'fleet'
  onToggleLanguage: () => void
  onOpenSettings: () => void
  onViewChange: (view: 'myskills' | 'explore' | 'manage' | 'fleet') => void
  t: TFunction
}

const Header = ({
  language,
  activeView,
  onToggleLanguage,
  onOpenSettings,
  onViewChange,
  t,
}: HeaderProps) => {
  return (
    // The header IS the title bar: its empty surface drags the window (Tauri
    // routes clicks on interactive children — buttons, nav — to them instead),
    // and a double-click on it toggles maximize, like a native title bar.
    <header className="skills-header" data-tauri-drag-region>
      <div className="header-left">
        <div className="brand-area" data-tauri-drag-region>
          <img className="brand-logo" src={logoIcon} alt="" aria-hidden="true" />
          <div className="brand-text-wrap">
            <div className="brand-text">
              Skill<span className="brand-text-accent">Basin</span>
            </div>
          </div>
        </div>
        <nav className="nav-tabs">
          <button
            className={`nav-tab${activeView === 'myskills' || activeView === 'detail' ? ' active' : ''}`}
            type="button"
            onClick={() => onViewChange('myskills')}
          >
            <Layers size={16} />
            {t('navMySkills')}
          </button>
          <button
            className={`nav-tab${activeView === 'explore' ? ' active' : ''}`}
            type="button"
            onClick={() => onViewChange('explore')}
          >
            <Search size={16} />
            {t('navExplore')}
          </button>
          <button
            className={`nav-tab${activeView === 'fleet' ? ' active' : ''}`}
            type="button"
            onClick={() => onViewChange('fleet')}
          >
            <Server size={16} />
            {t('navFleet')}
          </button>
          <button
            className={`nav-tab${activeView === 'manage' ? ' active' : ''}`}
            type="button"
            onClick={() => onViewChange('manage')}
          >
            <SlidersHorizontal size={16} />
            {t('navManageCenter')}
          </button>
        </nav>
      </div>
      <div className="header-actions">
        <button className="lang-btn" type="button" onClick={onToggleLanguage}>
          {t(`languageShort.${language === 'tr' ? 'tr' : language === 'zh' ? 'zh' : 'en'}`)}
        </button>
        <button className={`icon-btn${activeView === 'settings' ? ' active' : ''}`} type="button" onClick={onOpenSettings}>
          <Settings size={18} />
        </button>
        <WindowControls t={t} />
      </div>
    </header>
  )
}

export default memo(Header)
