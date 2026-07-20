import { memo, useCallback, useEffect, useMemo, useState } from 'react'
import { Download, Pin, PinOff } from 'lucide-react'
import { toast } from 'sonner'
import type { TFunction } from 'i18next'
import type {
  MachinePinsDto,
  PinSyncResultDto,
  PinTargetDto,
  PluginOverlapDto,
  SkillVersionDto,
  ToolOption,
} from './types'

type SkillVersionsPanelProps = {
  skillName: string
  installedTools: ToolOption[]
  invokeTauri: <T>(command: string, args?: Record<string, unknown>) => Promise<T>
  formatRelative: (ms: number | null | undefined) => string
  t: TFunction
}

type PinnedFor = {
  version: string
  target: PinTargetDto
}

const SkillVersionsPanel = ({
  skillName,
  installedTools,
  invokeTauri,
  t,
}: SkillVersionsPanelProps) => {
  const [versions, setVersions] = useState<SkillVersionDto[]>([])
  const [pins, setPins] = useState<MachinePinsDto | null>(null)
  const [loading, setLoading] = useState(true)
  const [pendingTool, setPendingTool] = useState<string | null>(null)
  const [exportingVersion, setExportingVersion] = useState<string | null>(null)
  const [overlapPlugin, setOverlapPlugin] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    try {
      const [versionList, machinePins] = await Promise.all([
        invokeTauri<SkillVersionDto[]>('list_skill_versions', { skillName }),
        invokeTauri<MachinePinsDto>('get_machine_pins'),
      ])
      setVersions(versionList)
      setPins(machinePins)
    } catch {
      setVersions([])
      setPins(null)
    } finally {
      setLoading(false)
    }
    // Best-effort, non-blocking: does an enabled Claude Code plugin already
    // provide this skill? If so, the pin creates a duplicate — flag it.
    try {
      const overlaps = await invokeTauri<PluginOverlapDto[]>('claude_plugin_overlaps')
      const hit = overlaps.find((o) => o.skill === skillName)
      setOverlapPlugin(hit ? hit.plugin : null)
    } catch {
      setOverlapPlugin(null)
    }
  }, [invokeTauri, skillName])

  useEffect(() => {
    void load()
  }, [load])

  const pinnedByTool = useMemo(() => {
    const map = new Map<string, PinnedFor>()
    if (!pins) return map
    for (const entry of pins.pins) {
      if (entry.skill !== skillName) continue
      for (const [tool, target] of Object.entries(entry.targets)) {
        // Disabled targets count as unpinned — same rule SkillCard applies.
        if (target.enabled) map.set(tool, { version: entry.version, target })
      }
    }
    return map
  }, [pins, skillName])

  const versionsByPinCount = useMemo(() => {
    const counts = new Map<string, string[]>()
    for (const [tool, pinned] of pinnedByTool) {
      const list = counts.get(pinned.version) ?? []
      list.push(tool)
      counts.set(pinned.version, list)
    }
    return counts
  }, [pinnedByTool])

  // A pin can be recorded while its on-disk sync is refused (unmanaged dir at
  // the target). Surface those failures instead of toasting success over them.
  const reportSyncFailures = useCallback(
    (result: PinSyncResultDto) => {
      const failures = result.results.filter((entry) => !entry.ok)
      for (const failure of failures) {
        toast.error(
          t('versions.syncRefused', {
            tool: failure.tool,
            reason: failure.error ?? t('versions.pinFailed'),
          }),
          { duration: 8000 },
        )
      }
      // The pin applied here but never reached the basin remote: other machines
      // will not see it. A warning, not an error — locally it did take effect.
      if (result.basin_warning) {
        toast.warning(t('versions.basinNotPublished', { reason: result.basin_warning }), {
          duration: 12000,
        })
      }
      return failures.length > 0
    },
    [t],
  )

  const handlePinChange = useCallback(
    async (tool: string, version: string) => {
      setPendingTool(tool)
      try {
        const result = await invokeTauri<PinSyncResultDto>('set_skill_pin', {
          skill: skillName,
          version,
          tool,
          target: { enabled: true, strategy: 'auto' },
        })
        setPins(result.pins)
        if (!reportSyncFailures(result)) {
          toast.success(t('versions.pinned', { tool, version }))
        }
        // A successful sync can still duplicate a plugin-provided skill; surface
        // that so the overlap doesn't stay silent.
        for (const entry of result.results) {
          if (entry.warning) toast.warning(entry.warning, { duration: 12000 })
        }
      } catch (err) {
        toast.error(err instanceof Error ? err.message : t('versions.pinFailed'))
      } finally {
        setPendingTool(null)
      }
    },
    [invokeTauri, reportSyncFailures, skillName, t],
  )

  const handleExport = useCallback(
    async (version: string) => {
      setExportingVersion(version)
      try {
        const suggested = await invokeTauri<string>('default_export_file_name', {
          skill: skillName,
          version,
        })
        const { save } = await import('@tauri-apps/plugin-dialog')
        const destination = await save({
          defaultPath: suggested,
          filters: [{ name: 'Zip archive', extensions: ['zip'] }],
        })
        if (!destination) return // user cancelled the save dialog
        const written = await invokeTauri<string>('export_skill_version', {
          skill: skillName,
          version,
          destination,
        })
        toast.success(t('versions.exported', { path: written }))
      } catch (err) {
        toast.error(err instanceof Error ? err.message : t('versions.exportFailed'))
      } finally {
        setExportingVersion(null)
      }
    },
    [invokeTauri, skillName, t],
  )

  const handleUnpin = useCallback(
    async (tool: string) => {
      setPendingTool(tool)
      try {
        const result = await invokeTauri<PinSyncResultDto>('unset_skill_pin', {
          skill: skillName,
          tool,
        })
        setPins(result.pins)
        if (!reportSyncFailures(result)) {
          toast.success(t('versions.unpinned', { tool }))
        }
      } catch (err) {
        toast.error(err instanceof Error ? err.message : t('versions.unpinFailed'))
      } finally {
        setPendingTool(null)
      }
    },
    [invokeTauri, reportSyncFailures, skillName, t],
  )

  if (loading) {
    return (
      <div className="versions-panel">
        <div className="detail-loading">
          <div className="detail-spinner" />
          {t('versions.loading')}
        </div>
      </div>
    )
  }

  if (versions.length === 0) {
    return (
      <div className="versions-panel">
        <div className="versions-empty">{t('versions.empty')}</div>
      </div>
    )
  }

  return (
    <div className="versions-panel">
      {overlapPlugin ? (
        <div className="versions-overlap-warning" role="status">
          {t('versions.pluginOverlap', { plugin: overlapPlugin })}
        </div>
      ) : null}
      <div className="versions-box">
        <h5 className="versions-box-title">{t('versions.title')}</h5>
        <ul className="versions-list">
          {versions.map((version) => {
            const pinnedTools = versionsByPinCount.get(version.label) ?? []
            return (
              <li className="versions-list-item" key={version.label}>
                <span className="mono versions-label">{version.label}</span>
                {version.isLatest ? (
                  <span className="chip chip-accent">{t('versions.latest')}</span>
                ) : null}
                {pinnedTools.length > 0 ? (
                  <span className="chip chip-pin" title={pinnedTools.join(', ')}>
                    <Pin size={11} />
                    {pinnedTools.length}
                  </span>
                ) : null}
                <span className="versions-date">{version.addedAt}</span>
                <button
                  type="button"
                  className="icon-btn versions-export"
                  disabled={exportingVersion === version.label}
                  onClick={() => void handleExport(version.label)}
                  aria-label={t('versions.export')}
                  title={t('versions.export')}
                >
                  <Download size={14} />
                </button>
              </li>
            )
          })}
        </ul>
      </div>

      <div className="versions-box">
        <h5 className="versions-box-title">{t('versions.pinMatrix')}</h5>
        <ul className="pin-matrix">
          {installedTools.map((tool) => {
            const pinned = pinnedByTool.get(tool.id)
            const isPending = pendingTool === tool.id
            return (
              <li className="pin-matrix-row" key={tool.id}>
                <span className="chip">{tool.label}</span>
                <span className="pin-matrix-arrow">→</span>
                <select
                  className="pin-matrix-select mono"
                  value={pinned?.version ?? ''}
                  disabled={isPending}
                  onChange={(event) => {
                    const next = event.target.value
                    if (next) {
                      void handlePinChange(tool.id, next)
                    } else if (pinned) {
                      // Choosing "off" is an unpin, same as the icon button.
                      void handleUnpin(tool.id)
                    }
                  }}
                >
                  <option value="">{t('versions.off')}</option>
                  {versions.map((version) => (
                    <option key={version.label} value={version.label}>
                      {version.label}
                    </option>
                  ))}
                </select>
                {pinned ? (
                  <button
                    type="button"
                    className="icon-btn pin-matrix-unpin"
                    disabled={isPending}
                    onClick={() => void handleUnpin(tool.id)}
                    aria-label={t('versions.unpin')}
                    title={t('versions.unpin')}
                  >
                    <PinOff size={14} />
                  </button>
                ) : null}
              </li>
            )
          })}
        </ul>
      </div>
    </div>
  )
}

export default memo(SkillVersionsPanel)
