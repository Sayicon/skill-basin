import { memo, useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { ArrowLeft, CheckCircle2, Clock3, LoaderCircle, X, XCircle } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { Update } from '@tauri-apps/plugin-updater'
import type {
  AutoUpdateConfigDto,
  AutoUpdateSkillProgressDto,
  GithubProxyConfigDto,
} from './types'
import {
  getAutoUpdateProgressCounts,
  getAutoUpdateTaskStatusKey,
  isAutoUpdatePossiblyStalled,
  parseAutoUpdateFailureItems,
} from './autoUpdateSettings'

type UpdateStatus = 'idle' | 'checking' | 'up-to-date' | 'available' | 'downloading' | 'done' | 'error'

type SettingsPageProps = {
  isTauri: boolean
  language: string
  storagePath: string
  gitCacheCleanupDays: number
  gitCacheTtlSecs: number
  autoUpdateConfig: AutoUpdateConfigDto | null
  themePreference: 'system' | 'light' | 'dark'
  githubToken: string
  githubProxyConfig: GithubProxyConfigDto
  onPickStoragePath: () => void
  onToggleLanguage: () => void
  onThemeChange: (nextTheme: 'system' | 'light' | 'dark') => void
  onGitCacheCleanupDaysChange: (nextDays: number) => void
  onGitCacheTtlSecsChange: (nextSecs: number) => void
  onClearGitCacheNow: () => void
  onGithubTokenChange: (token: string) => void
  onGithubProxyConfigChange: (enabled: boolean, port: number) => void
  onAutoUpdateConfigChange: (enabled: boolean, intervalHours: number) => void
  onRunAutoUpdateNow: () => void
  autoUpdateTriggering: boolean
  onBack: () => void
  t: TFunction
}

const SettingsPage = ({
  isTauri,
  language,
  storagePath,
  gitCacheCleanupDays,
  gitCacheTtlSecs,
  autoUpdateConfig,
  themePreference,
  onPickStoragePath,
  onToggleLanguage,
  onThemeChange,
  onGitCacheCleanupDaysChange,
  onGitCacheTtlSecsChange,
  onClearGitCacheNow,
  githubToken,
  onGithubTokenChange,
  githubProxyConfig,
  onGithubProxyConfigChange,
  onAutoUpdateConfigChange,
  onRunAutoUpdateNow,
  autoUpdateTriggering,
  onBack,
  t,
}: SettingsPageProps) => {
  const [localToken, setLocalToken] = useState(githubToken)
  useEffect(() => {
    setLocalToken(githubToken)
  }, [githubToken])
  const [localGithubProxyPort, setLocalGithubProxyPort] = useState(
    String(githubProxyConfig.port),
  )
  useEffect(() => {
    setLocalGithubProxyPort(String(githubProxyConfig.port))
  }, [githubProxyConfig.port])
  const [localAutoUpdateInterval, setLocalAutoUpdateInterval] = useState(24)
  useEffect(() => {
    setLocalAutoUpdateInterval(autoUpdateConfig?.interval_hours ?? 24)
  }, [autoUpdateConfig?.interval_hours])
  const [autoUpdateProgressOpen, setAutoUpdateProgressOpen] = useState(false)

  const [updateStatus, setUpdateStatus] = useState<UpdateStatus>('idle')
  const [updateVersion, setUpdateVersion] = useState<string | null>(null)
  const [updateError, setUpdateError] = useState<string | null>(null)
  const updateRef = useRef<Update | null>(null)

  const handleCheckUpdate = useCallback(async () => {
    if (!isTauri) return
    setUpdateStatus('checking')
    setUpdateError(null)
    try {
      const { check } = await import('@tauri-apps/plugin-updater')
      const update = await check()
      if (update) {
        updateRef.current = update
        setUpdateVersion(update.version)
        setUpdateStatus('available')
      } else {
        setUpdateStatus('up-to-date')
      }
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err))
      setUpdateStatus('error')
    }
  }, [isTauri])

  const handleInstallUpdate = useCallback(async () => {
    const update = updateRef.current
    if (!update) return
    setUpdateStatus('downloading')
    setUpdateError(null)
    try {
      await update.downloadAndInstall()
      setUpdateStatus('done')
    } catch (err) {
      setUpdateError(err instanceof Error ? err.message : String(err))
      setUpdateStatus('error')
    }
  }, [])

  const [appVersion, setAppVersion] = useState<string | null>(null)
  const versionText = useMemo(() => {
    if (!isTauri) return t('notAvailable')
    if (!appVersion) return t('unknown')
    return `v${appVersion}`
  }, [appVersion, isTauri, t])

  const loadAppVersion = useCallback(async () => {
    if (!isTauri) {
      setAppVersion(null)
      return
    }
    try {
      const { getVersion } = await import('@tauri-apps/api/app')
      const v = await getVersion()
      setAppVersion(v)
    } catch {
      setAppVersion(null)
    }
  }, [isTauri])

  useEffect(() => {
    void loadAppVersion()
    return () => { updateRef.current = null }
  }, [loadAppVersion])

  const autoUpdateEnabled = autoUpdateConfig?.enabled ?? false
  const autoUpdateInterval = autoUpdateConfig?.interval_hours ?? 24
  const autoUpdateHasLocalSkills = (autoUpdateConfig?.local_skill_count ?? 0) > 0
  const autoUpdateHasProtectedLocalSkills =
    (autoUpdateConfig?.protected_local_skill_count ?? 0) > 0
  const autoUpdateRunning = autoUpdateConfig?.last_status === 'running'
  const autoUpdateRunningLong =
    autoUpdateRunning &&
    Boolean(autoUpdateConfig?.last_run_at) &&
    Date.now() - (autoUpdateConfig?.last_run_at ?? 0) > 10 * 60 * 1000
  const autoUpdateStalled = isAutoUpdatePossiblyStalled(autoUpdateConfig, Date.now())
  const autoUpdateButtonBusy =
    (autoUpdateRunning && !autoUpdateStalled) || autoUpdateTriggering
  const autoUpdateLastRun = autoUpdateConfig?.last_run_at
    ? new Date(autoUpdateConfig.last_run_at).toLocaleString()
    : t('autoUpdateNeverRun')
  const autoUpdateStartedAt = autoUpdateConfig?.last_started_at
    ? new Date(autoUpdateConfig.last_started_at).toLocaleString()
    : t('autoUpdateNeverRun')
  const autoUpdateFinishedAt = autoUpdateConfig?.last_finished_at
    ? new Date(autoUpdateConfig.last_finished_at).toLocaleString()
    : autoUpdateRunning
      ? t('autoUpdateStatus.running')
      : t('autoUpdateNeverRun')
  const autoUpdateDuration =
    autoUpdateConfig?.last_started_at && autoUpdateConfig?.last_finished_at
      ? formatDuration(autoUpdateConfig.last_finished_at - autoUpdateConfig.last_started_at)
      : autoUpdateRunning && autoUpdateConfig?.last_started_at
        ? formatDuration(Date.now() - autoUpdateConfig.last_started_at)
        : t('autoUpdateNeverRun')
  const autoUpdateStatus = autoUpdateConfig?.last_status
    ? t(`autoUpdateStatus.${autoUpdateConfig.last_status}`)
    : t('autoUpdateStatus.none')
  const taskStatusKey = getAutoUpdateTaskStatusKey(
    autoUpdateEnabled,
    autoUpdateConfig?.task_registered ?? false,
  )
  const autoUpdateProgress = autoUpdateConfig?.progress
  const autoUpdateProgressForDisplay = useMemo(() => {
    const hasStructuredProgress =
      Boolean(autoUpdateProgress?.total) ||
      Boolean(autoUpdateProgress?.succeeded.length) ||
      Boolean(autoUpdateProgress?.failed.length) ||
      Boolean(autoUpdateProgress?.running) ||
      Boolean(autoUpdateProgress?.pending.length)
    if (autoUpdateProgress && hasStructuredProgress) {
      return autoUpdateProgress
    }

    return {
      total: autoUpdateConfig?.last_checked ?? 0,
      succeeded: [],
      failed: parseAutoUpdateFailureItems(autoUpdateConfig?.last_error),
      running: null,
      pending: [],
    }
  }, [
    autoUpdateConfig?.last_checked,
    autoUpdateConfig?.last_error,
    autoUpdateProgress,
  ])
  const rawAutoUpdateProgressCounts = getAutoUpdateProgressCounts(
    autoUpdateProgressForDisplay,
  )
  const autoUpdateCompletedCount =
    (autoUpdateConfig?.last_updated ?? 0) + (autoUpdateConfig?.last_failed ?? 0)
  const autoUpdateProgressCounts = {
    total: autoUpdateConfig?.last_checked || rawAutoUpdateProgressCounts.total,
    completed: autoUpdateCompletedCount || rawAutoUpdateProgressCounts.completed,
    succeeded: autoUpdateConfig?.last_updated ?? rawAutoUpdateProgressCounts.succeeded,
    failed: autoUpdateConfig?.last_failed ?? rawAutoUpdateProgressCounts.failed,
    active: Math.max(
      0,
      (autoUpdateConfig?.last_checked || rawAutoUpdateProgressCounts.total) -
        (autoUpdateCompletedCount || rawAutoUpdateProgressCounts.completed),
    ),
  }
  const autoUpdatePendingCount =
    autoUpdateProgressForDisplay.pending.length ||
    Math.max(
      0,
      autoUpdateProgressCounts.active -
        (autoUpdateProgressForDisplay.running ? 1 : 0),
    )
  const handleCopyAutoUpdateError = useCallback(async () => {
    if (!autoUpdateConfig?.last_error) return
    await navigator.clipboard.writeText(autoUpdateConfig.last_error)
  }, [autoUpdateConfig?.last_error])
  const renderProgressItems = (
    items: AutoUpdateSkillProgressDto[],
    emptyKey: string,
    showReason = false,
  ) => {
    if (items.length === 0) {
      return <div className="auto-update-progress-empty">{t(emptyKey)}</div>
    }

    return (
      <div className="auto-update-progress-list">
        {items.map((item) => (
          <div className="auto-update-progress-item" key={item.skill_id}>
            <div className="auto-update-progress-name">{item.name || item.skill_id}</div>
            {showReason && item.reason ? (
              <div className="auto-update-progress-reason">{item.reason}</div>
            ) : null}
          </div>
        ))}
      </div>
    )
  }

  return (
    <div className="settings-page">
      <div className="detail-header">
        <button className="detail-back-btn" type="button" onClick={onBack}>
          <ArrowLeft size={16} />
          {t('detail.back')}
        </button>
        <div className="detail-skill-name">{t('settings')}</div>
      </div>
      <div className="settings-page-body">
        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-language">
            {t('interfaceLanguage')}
          </label>
          <div className="settings-select-wrap">
            <select
              id="settings-language"
              className="settings-select"
              value={language}
              onChange={(event) => {
                if (event.target.value !== language) {
                  onToggleLanguage()
                }
              }}
            >
              <option value="en">{t('languageOptions.en')}</option>
              <option value="zh">{t('languageOptions.zh')}</option>
            </select>
            <svg
              className="settings-select-caret"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              aria-hidden="true"
            >
              <path d="M6 9l6 6 6-6" />
            </svg>
          </div>
        </div>

        <div className="settings-field">
          <label className="settings-label" id="settings-theme-label">
            {t('themeMode')}
          </label>
          <div className="settings-theme-options" role="group" aria-labelledby="settings-theme-label">
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === 'system' ? 'active' : ''
              }`}
              aria-pressed={themePreference === 'system'}
              onClick={() => onThemeChange('system')}
            >
              {t('themeOptions.system')}
            </button>
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === 'light' ? 'active' : ''
              }`}
              aria-pressed={themePreference === 'light'}
              onClick={() => onThemeChange('light')}
            >
              {t('themeOptions.light')}
            </button>
            <button
              type="button"
              className={`settings-theme-btn ${
                themePreference === 'dark' ? 'active' : ''
              }`}
              aria-pressed={themePreference === 'dark'}
              onClick={() => onThemeChange('dark')}
            >
              {t('themeOptions.dark')}
            </button>
          </div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-storage">
            {t('skillsStoragePath')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-storage"
              className="settings-input mono"
              value={storagePath}
              readOnly
            />
            <button
              className="btn btn-secondary settings-browse"
              type="button"
              onClick={onPickStoragePath}
            >
              {t('browse')}
            </button>
          </div>
          <div className="settings-helper">{t('skillsStorageHint')}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-git-cache-days">
            {t('gitCacheCleanupDays')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-git-cache-days"
              className="settings-input"
              type="number"
              min={0}
              max={3650}
              step={1}
              value={gitCacheCleanupDays}
              onChange={(event) => {
                const next = Number(event.target.value)
                if (!Number.isNaN(next)) {
                  onGitCacheCleanupDaysChange(next)
                }
              }}
            />
            <button
              className="btn btn-secondary settings-browse"
              type="button"
              onClick={onClearGitCacheNow}
            >
              {t('cleanNow')}
            </button>
          </div>
          <div className="settings-helper">{t('gitCacheCleanupHint')}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-git-cache-ttl">
            {t('gitCacheTtlSecs')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-git-cache-ttl"
              className="settings-input"
              type="number"
              min={0}
              max={3600}
              step={1}
              value={gitCacheTtlSecs}
              onChange={(event) => {
                const next = Number(event.target.value)
                if (!Number.isNaN(next)) {
                  onGitCacheTtlSecsChange(next)
                }
              }}
            />
          </div>
          <div className="settings-helper">{t('gitCacheTtlHint')}</div>
        </div>

        <div className="settings-field">
          <label className="settings-label" htmlFor="settings-github-token">
            {t('githubToken')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-github-token"
              className="settings-input mono"
              type="password"
              placeholder={t('githubTokenPlaceholder')}
              value={localToken}
              onChange={(e) => setLocalToken(e.target.value)}
              onBlur={() => {
                if (localToken !== githubToken) {
                  onGithubTokenChange(localToken)
                }
              }}
            />
          </div>
          <div className="settings-helper">{t('githubTokenHint')}</div>
        </div>

        <div className="settings-field">
          <div className="settings-item">
            <div className="settings-item-info">
              <div className="settings-item-title">{t('networkProxy')}</div>
              <div className="settings-item-desc">{t('networkProxyHint')}</div>
            </div>
            <button
              type="button"
              className={`settings-toggle${githubProxyConfig.enabled ? ' checked' : ''}`}
              aria-pressed={githubProxyConfig.enabled}
              onClick={() => {
                const nextPort = Number(localGithubProxyPort)
                onGithubProxyConfigChange(
                  !githubProxyConfig.enabled,
                  Number.isNaN(nextPort) ? githubProxyConfig.port : nextPort,
                )
              }}
            >
              <span className="settings-toggle-knob" />
            </button>
          </div>
          <label className="settings-label" htmlFor="settings-github-proxy-port">
            {t('networkProxyPort')}
          </label>
          <div className="settings-input-row">
            <input
              id="settings-github-proxy-port"
              className="settings-input mono"
              type="number"
              min={1}
              max={65535}
              step={1}
              value={localGithubProxyPort}
              onChange={(e) => setLocalGithubProxyPort(e.target.value)}
              onBlur={() => {
                const nextPort = Number(localGithubProxyPort)
                if (
                  githubProxyConfig.enabled &&
                  !Number.isNaN(nextPort) &&
                  nextPort !== githubProxyConfig.port
                ) {
                  onGithubProxyConfigChange(true, nextPort)
                }
              }}
            />
          </div>
          <div className="settings-helper">
            {githubProxyConfig.auto_detected
              ? t('networkProxyAutoDetected')
              : t('networkProxyPortHint')}
          </div>
        </div>

        <div className="settings-field">
          <label className="settings-label">{t('autoUpdateSkills')}</label>
          <div className="settings-item">
            <div className="settings-item-info">
              <div className="settings-item-title">{t('autoUpdateSystemTask')}</div>
              <div className="settings-item-desc">
                {t('autoUpdateSystemTaskDesc')}
              </div>
            </div>
            <button
              type="button"
              className={`settings-toggle${autoUpdateEnabled ? ' checked' : ''}`}
              aria-pressed={autoUpdateEnabled}
              onClick={() => {
                onAutoUpdateConfigChange(
                  !autoUpdateEnabled,
                  localAutoUpdateInterval,
                )
              }}
            >
              <span className="settings-toggle-knob" />
            </button>
          </div>
          <div className="settings-input-row">
            <input
              id="settings-auto-update-hours"
              className="settings-input"
              type="number"
              min={1}
              max={720}
              step={1}
              value={localAutoUpdateInterval}
              onChange={(event) => {
                const next = Number(event.target.value)
                if (!Number.isNaN(next)) {
                  setLocalAutoUpdateInterval(next)
                }
              }}
              onBlur={() => {
                if (localAutoUpdateInterval !== autoUpdateInterval) {
                  onAutoUpdateConfigChange(
                    autoUpdateEnabled,
                    localAutoUpdateInterval,
                  )
                }
              }}
            />
            <button
              className="btn btn-secondary settings-browse"
              type="button"
              disabled={autoUpdateButtonBusy}
              onClick={onRunAutoUpdateNow}
            >
              {autoUpdateButtonBusy
                ? t('autoUpdateRunningButton')
                : autoUpdateStalled
                  ? t('autoUpdateRetryUpdate')
                  : t('autoUpdateRunNow')}
            </button>
          </div>
          <div className="settings-helper">
            {t('autoUpdateIntervalHint', { hours: autoUpdateInterval })}
          </div>
          {autoUpdateHasLocalSkills ? (
            <div className="settings-helper">
              {t(
                autoUpdateHasProtectedLocalSkills
                  ? 'autoUpdateLocalPermissionProtectedHint'
                  : 'autoUpdateLocalPermissionHint',
              )}
            </div>
          ) : null}
          <div className="settings-helper">
            {t('autoUpdateTaskStatus', { status: t(taskStatusKey) })}
          </div>
          {taskStatusKey === 'autoUpdateTaskNeedsAttention' ? (
            <div className="settings-helper">
              {t('autoUpdateTaskNeedsAttentionHint')}
            </div>
          ) : null}
          <div className="settings-helper">
            {t('autoUpdateLastRun', {
              time: autoUpdateLastRun,
              status: autoUpdateStatus,
              checked: autoUpdateConfig?.last_checked ?? 0,
              updated: autoUpdateConfig?.last_updated ?? 0,
              failed: autoUpdateConfig?.last_failed ?? 0,
            })}
          </div>
          <div className="settings-helper">
            <button
              className="btn btn-secondary btn-sm"
              type="button"
              onClick={() => setAutoUpdateProgressOpen(true)}
            >
              {t('autoUpdateViewProgress')}
            </button>
          </div>
          {autoUpdateStalled ? (
            <div className="settings-helper">
              {t('autoUpdateStalledHint')}
            </div>
          ) : autoUpdateRunningLong ? (
            <div className="settings-helper">
              {t('autoUpdateLongRunningHint')}
            </div>
          ) : null}
          {autoUpdateProgressOpen ? (
            <div
              className="modal-backdrop"
              onClick={() => setAutoUpdateProgressOpen(false)}
            >
              <div
                className="modal modal-lg auto-update-progress-modal"
                onClick={(event) => event.stopPropagation()}
              >
                <div className="modal-header">
                  <div className="modal-title">{t('autoUpdateProgressTitle')}</div>
                  <button
                    className="modal-close"
                    type="button"
                    onClick={() => setAutoUpdateProgressOpen(false)}
                    aria-label={t('close')}
                  >
                    <X size={18} />
                  </button>
                </div>
                <div className="modal-body">
                  <div className="auto-update-progress-stats">
                    <div>
                      <span>{t('autoUpdateProgressTotal')}</span>
                      <strong>{autoUpdateProgressCounts.total}</strong>
                    </div>
                    <div>
                      <span>{t('autoUpdateProgressSucceeded')}</span>
                      <strong>{autoUpdateProgressCounts.succeeded}</strong>
                    </div>
                    <div>
                      <span>{t('autoUpdateProgressFailed')}</span>
                      <strong>{autoUpdateProgressCounts.failed}</strong>
                    </div>
                    <div>
                      <span>{t('autoUpdateProgressActive')}</span>
                      <strong>{autoUpdateProgressCounts.active}</strong>
                    </div>
                  </div>
                  <div className="auto-update-progress-runtime">
                    <div>
                      <span>{t('autoUpdateProgressStartedAt')}</span>
                      <strong>{autoUpdateStartedAt}</strong>
                    </div>
                    <div>
                      <span>{t('autoUpdateProgressFinishedAt')}</span>
                      <strong>{autoUpdateFinishedAt}</strong>
                    </div>
                    <div>
                      <span>{t('autoUpdateProgressDuration')}</span>
                      <strong>{autoUpdateDuration}</strong>
                    </div>
                  </div>

                  <section className="auto-update-progress-section">
                    <h3>
                      <LoaderCircle size={16} />
                      {t('autoUpdateProgressRunning')}
                    </h3>
                    {autoUpdateProgressForDisplay.running ? (
                      <div className="auto-update-progress-list">
                        <div className="auto-update-progress-item">
                          <div className="auto-update-progress-name">
                            {autoUpdateProgressForDisplay.running.name ||
                              autoUpdateProgressForDisplay.running.skill_id}
                          </div>
                        </div>
                      </div>
                    ) : (
                      <div className="auto-update-progress-empty">
                        {autoUpdateProgressCounts.active > 0
                          ? t('autoUpdateProgressWaitingToStart')
                          : t('autoUpdateProgressNoRunning')}
                      </div>
                    )}
                  </section>

                  <section className="auto-update-progress-section">
                    <h3>
                      <XCircle size={16} />
                      {t('autoUpdateProgressFailed')}
                    </h3>
                    {renderProgressItems(
                      autoUpdateProgressForDisplay.failed,
                      'autoUpdateProgressNoFailed',
                      true,
                    )}
                  </section>

                  <section className="auto-update-progress-section">
                    <h3>
                      <CheckCircle2 size={16} />
                      {t('autoUpdateProgressSucceeded')}
                    </h3>
                    <div className="auto-update-progress-empty">
                      {autoUpdateProgressCounts.succeeded > 0
                        ? t('autoUpdateProgressSucceededSummary', {
                            count: autoUpdateProgressCounts.succeeded,
                          })
                        : t('autoUpdateProgressNoSucceeded')}
                    </div>
                  </section>

                  <section className="auto-update-progress-section">
                    <h3>
                      <Clock3 size={16} />
                      {t('autoUpdateProgressPending')}
                    </h3>
                    <div className="auto-update-progress-empty">
                      {autoUpdatePendingCount > 0
                        ? t('autoUpdateProgressPendingSummary', {
                            count: autoUpdatePendingCount,
                          })
                        : t('autoUpdateProgressNoPending')}
                    </div>
                  </section>

                  {autoUpdateConfig?.last_error ? (
                    <div className="modal-actions">
                      <button
                        className="btn btn-secondary btn-sm"
                        type="button"
                        onClick={() => {
                          void handleCopyAutoUpdateError()
                        }}
                      >
                        {t('copyDetails')}
                      </button>
                    </div>
                  ) : null}
                </div>
              </div>
            </div>
          ) : null}
        </div>

        <div className="settings-field settings-update-section">
          <label className="settings-label">{t('appUpdates')}</label>
          <div className="settings-version-row">
            <span className="settings-version-text">
              {t('appName')} {versionText}
            </span>
            {isTauri && updateStatus === 'idle' && (
              <button
                className="btn btn-secondary btn-sm"
                type="button"
                onClick={handleCheckUpdate}
              >
                {t('checkForUpdates')}
              </button>
            )}
            {updateStatus === 'checking' && (
              <span className="settings-update-status">{t('checkingUpdates')}</span>
            )}
            {updateStatus === 'up-to-date' && (
              <span className="settings-update-status settings-update-ok">{t('updateNotAvailable')}</span>
            )}
          </div>
          {updateStatus === 'available' && (
            <div className="settings-update-available">
              <span>{t('updateAvailableWithVersion', { version: updateVersion })}</span>
              <button
                className="btn btn-primary btn-sm"
                type="button"
                onClick={handleInstallUpdate}
              >
                {t('downloadAndInstall')}
              </button>
            </div>
          )}
          {updateStatus === 'downloading' && (
            <div className="settings-update-status">{t('installingUpdate')}</div>
          )}
          {updateStatus === 'done' && (
            <div className="settings-update-ok">{t('updateInstalledRestart')}</div>
          )}
          {updateStatus === 'error' && (
            <div className="settings-update-error">
              <span>{updateError}</span>
              <button
                className="btn btn-secondary btn-sm"
                type="button"
                onClick={handleCheckUpdate}
              >
                {t('checkForUpdates')}
              </button>
            </div>
          )}
          <div className="settings-helper">{t('updateHint')}</div>
        </div>

      </div>
    </div>
  )
}

function formatDuration(durationMs: number) {
  const totalSeconds = Math.max(0, Math.round(durationMs / 1000))
  const hours = Math.floor(totalSeconds / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  if (hours > 0) {
    return `${hours}h ${minutes}m ${seconds}s`
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds}s`
  }
  return `${seconds}s`
}

export default memo(SettingsPage)
