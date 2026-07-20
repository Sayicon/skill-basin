import { memo, useState } from 'react'
import { Copy } from 'lucide-react'
import type { TFunction } from 'i18next'

export type NameConflict = {
  /** Existing skill already holding the name. */
  existingId: string
  name: string
  centralPath: string
}

type NameConflictModalProps = {
  conflict: NameConflict | null
  loading: boolean
  onRequestClose: () => void
  onUpdateExisting: () => void
  onRename: (newName: string) => void
  t: TFunction
}

/**
 * A skill's name is its sync identity, so two enabled skills cannot share one.
 * Refusing the install is correct, but refusing it with only an error message
 * leaves the user stuck: what they usually meant was "update the one I have".
 * So the block is a fork, not a dead end.
 *
 * "Update the existing one" pulls the skill you already have from ITS OWN
 * recorded source, not from the one being installed — so it can never quietly
 * change what that skill is. The label says so, because "update" next to an
 * install prompt otherwise reads as "install this over that".
 */
const NameConflictModal = ({
  conflict,
  loading,
  onRequestClose,
  onUpdateExisting,
  onRename,
  t,
}: NameConflictModalProps) => {
  const [newName, setNewName] = useState('')
  const [seenId, setSeenId] = useState<string | null>(null)

  // Seed the field per conflict so a previous attempt's text never leaks into
  // the next one. Adjusted during render rather than in an effect: an effect
  // here would render once with the stale name and then re-render.
  if (conflict && conflict.existingId !== seenId) {
    setSeenId(conflict.existingId)
    setNewName(`${conflict.name}-2`)
  }

  if (!conflict) return null

  const renameValid = newName.trim().length > 0 && newName.trim() !== conflict.name

  return (
    <div className="modal-backdrop" onClick={onRequestClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()} role="dialog" aria-modal="true">
        <div className="modal-header">
          <div className="modal-title">{t('nameConflict.title', { name: conflict.name })}</div>
        </div>
        <div className="modal-body">
          <div className="modal-license-warning" role="alert">
            <Copy size={15} />
            <span>{t('nameConflict.hint')}</span>
          </div>
          <p className="modal-license-target mono">{conflict.centralPath}</p>

          <label className="settings-label" htmlFor="name-conflict-rename">
            {t('nameConflict.renameLabel')}
          </label>
          <input
            id="name-conflict-rename"
            className="settings-input mono"
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            disabled={loading}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && renameValid && !loading) onRename(newName.trim())
            }}
          />
        </div>
        <div className="modal-footer">
          <button className="btn btn-secondary" onClick={onRequestClose} disabled={loading}>
            {t('cancel')}
          </button>
          <button className="btn btn-secondary" onClick={onUpdateExisting} disabled={loading}>
            {t('nameConflict.updateExisting')}
          </button>
          <button
            className="btn btn-primary"
            onClick={() => onRename(newName.trim())}
            disabled={loading || !renameValid}
          >
            {t('nameConflict.installAsNew')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default memo(NameConflictModal)
