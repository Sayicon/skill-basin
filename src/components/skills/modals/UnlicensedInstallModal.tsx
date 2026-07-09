import { memo } from 'react'
import { ShieldAlert } from 'lucide-react'
import type { TFunction } from 'i18next'

type UnlicensedInstallModalProps = {
  open: boolean
  loading: boolean
  skillName?: string
  onRequestClose: () => void
  onConfirm: () => void
  t: TFunction
}

/**
 * Explore installs the moment its button is clicked, so a skill whose source
 * declares no license gets one deliberate stop before anything is fetched.
 * Without a license, the user has no permission to use or redistribute it —
 * that is a decision to make knowingly, not a detail to discover later.
 */
const UnlicensedInstallModal = ({
  open,
  loading,
  skillName,
  onRequestClose,
  onConfirm,
  t,
}: UnlicensedInstallModalProps) => {
  if (!open) return null

  return (
    <div className="modal-backdrop" onClick={onRequestClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div className="modal-header">
          <div className="modal-title">{t('explore.licenseMissing')}</div>
        </div>
        <div className="modal-body">
          <div className="modal-license-warning" role="alert">
            <ShieldAlert size={15} />
            <span>{t('explore.licenseMissingHint')}</span>
          </div>
          {skillName ? (
            <p className="modal-license-target">
              {t('explore.installAnyway', { name: skillName })}
            </p>
          ) : null}
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onRequestClose} disabled={loading}>
            {t('cancel')}
          </button>
          <button className="btn btn-primary" onClick={onConfirm} disabled={loading}>
            {t('install')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default memo(UnlicensedInstallModal)
