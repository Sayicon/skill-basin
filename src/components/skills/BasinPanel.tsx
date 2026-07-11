import { useCallback, useEffect, useState } from 'react'
import { Database, KeyRound, Plug, Plus, RefreshCw } from 'lucide-react'
import type { TFunction } from 'i18next'
import { toast } from 'sonner'

export type BasinStatus = {
  state: 'notConfigured' | 'broken' | 'ok'
  path?: string | null
  reason?: string | null
  remoteUrl?: string | null
}

type SecretStatus = {
  name: string
  state: 'keychain' | 'envFile' | 'missing'
}

type Invoke = <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>

/* ── Havuz bağlama / oluşturma formu (kart içinde ve sihirbazda ortak) ── */

type SetupFormProps = {
  invokeTauri: Invoke
  onDone: (status: BasinStatus) => void
  t: TFunction
}

export const BasinSetupForm = ({ invokeTauri, onDone, t }: SetupFormProps) => {
  const [mode, setMode] = useState<'connect' | 'create'>('connect')
  const [repoUrl, setRepoUrl] = useState('')
  const [dest, setDest] = useState('~/skillbasin-havuz')
  const [remoteUrl, setRemoteUrl] = useState('')
  const [busy, setBusy] = useState(false)

  const submit = async () => {
    setBusy(true)
    try {
      const status =
        mode === 'connect'
          ? await invokeTauri<BasinStatus>('basin_connect', { repoUrl, dest })
          : await invokeTauri<BasinStatus>('basin_create', {
              dest,
              remoteUrl: remoteUrl.trim() || null,
            })
      toast.success(t('basin.connected'))
      onDone(status)
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err))
    } finally {
      setBusy(false)
    }
  }

  return (
    <div className="basin-setup">
      <div className="basin-mode-tabs" role="tablist">
        <button
          type="button"
          role="tab"
          aria-selected={mode === 'connect'}
          className={`basin-mode-tab${mode === 'connect' ? ' active' : ''}`}
          onClick={() => setMode('connect')}
        >
          <Plug size={14} /> {t('basin.modeConnect')}
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={mode === 'create'}
          className={`basin-mode-tab${mode === 'create' ? ' active' : ''}`}
          onClick={() => setMode('create')}
        >
          <Plus size={14} /> {t('basin.modeCreate')}
        </button>
      </div>

      {mode === 'connect' ? (
        <div className="settings-field">
          <label className="settings-label">{t('basin.repoUrl')}</label>
          <input
            className="settings-input mono"
            value={repoUrl}
            placeholder="git@github.com:you/your-basin.git"
            onChange={(e) => setRepoUrl(e.target.value)}
          />
        </div>
      ) : (
        <div className="settings-field">
          <label className="settings-label">{t('basin.remoteOptional')}</label>
          <input
            className="settings-input mono"
            value={remoteUrl}
            placeholder="git@github.com:you/your-basin.git"
            onChange={(e) => setRemoteUrl(e.target.value)}
          />
        </div>
      )}
      <div className="settings-field">
        <label className="settings-label">{t('basin.localDir')}</label>
        <input
          className="settings-input mono"
          value={dest}
          onChange={(e) => setDest(e.target.value)}
        />
      </div>
      <button
        type="button"
        className="basin-submit"
        disabled={busy || (mode === 'connect' && !repoUrl.trim()) || !dest.trim()}
        onClick={() => void submit()}
      >
        {busy
          ? t('basin.working')
          : mode === 'connect'
            ? t('basin.doConnect')
            : t('basin.doCreate')}
      </button>
    </div>
  )
}

/* ── İlk açılış sihirbazı ─────────────────────────────────────────────── */

type WizardProps = {
  invokeTauri: Invoke
  status: BasinStatus
  onDone: (status: BasinStatus) => void
  onSkip: () => void
  t: TFunction
}

export const BasinSetupModal = ({ invokeTauri, status, onDone, onSkip, t }: WizardProps) => (
  <div className="modal-overlay" role="dialog" aria-modal="true">
    <div className="modal basin-wizard">
      <h2>{t('basin.wizardTitle')}</h2>
      <p className="basin-wizard-help">
        {status.state === 'broken' ? t('basin.wizardBroken') : t('basin.wizardHelp')}
      </p>
      {status.state === 'broken' ? (
        <div className="basin-broken-detail mono">
          {status.path}
          {status.reason ? ` — ${status.reason}` : ''}
        </div>
      ) : null}
      <BasinSetupForm invokeTauri={invokeTauri} onDone={onDone} t={t} />
      <button type="button" className="basin-skip" onClick={onSkip}>
        {t('basin.wizardSkip')}
      </button>
    </div>
  </div>
)

/* ── Settings kartları ────────────────────────────────────────────────── */

type CardsProps = {
  invokeTauri: Invoke
  t: TFunction
}

export const BasinSettingsCard = ({ invokeTauri, t }: CardsProps) => {
  const [status, setStatus] = useState<BasinStatus | null>(null)
  const [showSetup, setShowSetup] = useState(false)

  const load = useCallback(() => {
    invokeTauri<BasinStatus>('basin_status')
      .then(setStatus)
      .catch(() => setStatus(null))
  }, [invokeTauri])

  useEffect(() => {
    load()
  }, [load])

  return (
    <section className="settings-card">
      <div className="settings-card-head">
        <span className="settings-card-icon">
          <Database size={18} />
        </span>
        <div>
          <h2>{t('basin.cardTitle')}</h2>
          <p>{t('basin.cardDesc')}</p>
        </div>
      </div>
      <div className="settings-card-body">
        {status?.state === 'ok' && !showSetup ? (
          <>
            <div className="basin-kv">
              <span>{t('basin.localDir')}</span>
              <span className="mono">{status.path}</span>
            </div>
            <div className="basin-kv">
              <span>{t('basin.remote')}</span>
              <span className="mono">{status.remoteUrl ?? t('basin.noRemote')}</span>
            </div>
            <div className="basin-kv">
              <span>{t('basin.state')}</span>
              <span className="basin-chip ok">{t('basin.stateOk')}</span>
            </div>
            <button type="button" className="basin-skip" onClick={() => setShowSetup(true)}>
              {t('basin.changeBasin')}
            </button>
          </>
        ) : (
          <>
            {status?.state === 'broken' ? (
              <div className="basin-broken-detail mono">
                {status.path}
                {status.reason ? ` — ${status.reason}` : ''}
              </div>
            ) : null}
            <BasinSetupForm
              invokeTauri={invokeTauri}
              onDone={(s) => {
                setStatus(s)
                setShowSetup(false)
              }}
              t={t}
            />
          </>
        )}
      </div>
    </section>
  )
}

export const SecretsSettingsCard = ({ invokeTauri, t }: CardsProps) => {
  const [secrets, setSecrets] = useState<SecretStatus[]>([])
  const [editing, setEditing] = useState<string | null>(null)
  const [value, setValue] = useState('')

  const load = useCallback(() => {
    invokeTauri<SecretStatus[]>('secrets_status')
      .then(setSecrets)
      .catch(() => setSecrets([]))
  }, [invokeTauri])

  useEffect(() => {
    load()
  }, [load])

  const save = async (name: string) => {
    try {
      await invokeTauri('set_secret', { name, value })
      toast.success(t('secrets.saved'))
      setEditing(null)
      setValue('')
      load()
    } catch (err) {
      toast.error(err instanceof Error ? err.message : String(err))
    }
  }

  return (
    <section className="settings-card">
      <div className="settings-card-head">
        <span className="settings-card-icon">
          <KeyRound size={18} />
        </span>
        <div>
          <h2>{t('secrets.cardTitle')}</h2>
          <p>{t('secrets.cardDesc')}</p>
        </div>
        <button type="button" className="icon-btn" onClick={() => load()} aria-label={t('fleet.refresh')}>
          <RefreshCw size={15} />
        </button>
      </div>
      <div className="settings-card-body">
        {secrets.length === 0 ? (
          <p className="basin-empty">{t('secrets.none')}</p>
        ) : (
          <ul className="secrets-list">
            {secrets.map((s) => (
              <li key={s.name}>
                <span className="mono">{s.name}</span>
                {s.state === 'missing' ? (
                  editing === s.name ? (
                    <span className="secrets-edit">
                      <input
                        className="settings-input mono"
                        type="password"
                        value={value}
                        onChange={(e) => setValue(e.target.value)}
                        placeholder={t('secrets.valuePlaceholder')}
                      />
                      <button
                        type="button"
                        className="basin-submit small"
                        disabled={!value}
                        onClick={() => void save(s.name)}
                      >
                        {t('secrets.save')}
                      </button>
                    </span>
                  ) : (
                    <button
                      type="button"
                      className="basin-chip err clickable"
                      onClick={() => {
                        setEditing(s.name)
                        setValue('')
                      }}
                    >
                      {t('secrets.missing')}
                    </button>
                  )
                ) : (
                  <span className="basin-chip ok">
                    {s.state === 'keychain' ? t('secrets.keychain') : t('secrets.envFile')}
                  </span>
                )}
              </li>
            ))}
          </ul>
        )}
      </div>
    </section>
  )
}
