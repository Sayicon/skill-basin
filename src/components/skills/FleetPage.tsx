import { useCallback, useEffect, useState } from 'react'
import { Copy, Monitor, RefreshCw, Server } from 'lucide-react'
import type { TFunction } from 'i18next'
import { toast } from 'sonner'

export type FleetStatusAction = {
  skill: string
  tool: string
  action: string
  ok: boolean
  error?: string | null
}

export type FleetStatusReport = {
  schemaVersion: number
  machine: string
  appliedAtEpoch: number
  ok: boolean
  actions: FleetStatusAction[]
}

export type FleetMachine = {
  machine: string
  isCurrent: boolean
  pinnedByTool: Record<string, number>
  status?: FleetStatusReport | null
  statusError?: string | null
}

type FleetPageProps = {
  invokeTauri: <T>(cmd: string, args?: Record<string, unknown>) => Promise<T>
  language: string
  t: TFunction
}

const INSTALL_SNIPPET = `git clone <your-app-repo> skillbasin && cd skillbasin/hub-agent
cargo build --release
cat > ~/.skillbasin/agent.json <<'EOF'
{
  "repo": "<your-basin-git-url>",
  "machine": "<machine-id>",
  "basinDir": "~/.skillbasin/basin"
}
EOF
./target/release/skillbasin-agent init
./target/release/skillbasin-agent apply   # cron: */15 * * * *`

const AGENT_INSTRUCTIONS = `You are setting up skillbasin-agent on this machine.
1. Install rust + git if missing.
2. Clone the SkillBasin app repo, build hub-agent: cargo build --release (in hub-agent/).
3. Write ~/.skillbasin/agent.json with: repo (the basin git URL), machine (this machine's id in the basin), basinDir (where the clone should live), and optional toolDirs overrides for non-standard skill mounts.
4. Run skillbasin-agent init, then skillbasin-agent apply.
5. Schedule apply on cron (every 15 min). Exit code 1 means a pin failed — read machines/<id>/status.json.
6. If a tool caches skills (e.g. Hermes), trigger its reload after apply (/reload-skills).`

const formatAppliedAt = (epoch: number, language: string) =>
  new Date(epoch * 1000).toLocaleString(language === 'tr' ? 'tr-TR' : language === 'zh' ? 'zh-CN' : 'en-US')

const FleetPage = ({ invokeTauri, language, t }: FleetPageProps) => {
  const [machines, setMachines] = useState<FleetMachine[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [expanded, setExpanded] = useState<string | null>(null)

  const load = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      setMachines(await invokeTauri<FleetMachine[]>('fleet_machines'))
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [invokeTauri])

  useEffect(() => {
    void load()
  }, [load])

  const copyText = async (text: string, doneKey: string) => {
    try {
      await navigator.clipboard.writeText(text)
      toast.success(t(doneKey))
    } catch {
      toast.error(t('copyFailed'))
    }
  }

  return (
    <div className="fleet-page">
      <div className="fleet-header">
        <div>
          <h1>{t('fleet.title')}</h1>
          <p>{t('fleet.help')}</p>
        </div>
        <button className="fleet-refresh" type="button" onClick={() => void load()} disabled={loading}>
          <RefreshCw size={15} className={loading ? 'spin' : undefined} />
          {t('fleet.refresh')}
        </button>
      </div>

      {error ? <div className="fleet-error-banner">{error}</div> : null}

      <div className="fleet-grid">
        {machines.map((m) => {
          const failed = m.status ? m.status.actions.filter((a) => !a.ok) : []
          const health = m.statusError
            ? 'corrupt'
            : !m.status
              ? 'silent'
              : m.status.ok
                ? 'ok'
                : 'failing'
          const totalPins = Object.values(m.pinnedByTool).reduce((a, b) => a + b, 0)
          return (
            <div key={m.machine} className="fleet-card">
              <div className="fleet-card-head">
                <h3>
                  {m.isCurrent ? <Monitor size={15} /> : <Server size={15} />}
                  <span className="mono">{m.machine}</span>
                  {m.isCurrent ? <span className="fleet-chip current">{t('fleet.thisMachine')}</span> : null}
                </h3>
                {health === 'ok' ? (
                  <span className="fleet-chip ok">{t('fleet.healthy')}</span>
                ) : health === 'failing' ? (
                  <button
                    className="fleet-chip err clickable"
                    type="button"
                    onClick={() => setExpanded(expanded === m.machine ? null : m.machine)}
                  >
                    {t('fleet.errors', { count: failed.length })}
                  </button>
                ) : health === 'corrupt' ? (
                  <span className="fleet-chip err">{t('fleet.corruptStatus')}</span>
                ) : (
                  <span className="fleet-chip muted">{t('fleet.noReport')}</span>
                )}
              </div>
              <p className="fleet-meta mono">
                {m.status
                  ? t('fleet.lastApplied', {
                      time: formatAppliedAt(m.status.appliedAtEpoch, language),
                    })
                  : t('fleet.neverApplied')}
                {' · '}
                {t('fleet.pinCount', { count: totalPins })}
              </p>
              <div className="fleet-tools">
                {Object.entries(m.pinnedByTool).map(([tool, count]) => (
                  <span key={tool} className="fleet-chip tool mono">
                    {tool} {count}
                  </span>
                ))}
                {totalPins === 0 ? <span className="fleet-meta">{t('fleet.noPins')}</span> : null}
              </div>
              {m.statusError ? <div className="fleet-error-detail mono">{m.statusError}</div> : null}
              {expanded === m.machine && failed.length > 0 ? (
                <ul className="fleet-failures">
                  {failed.map((a, i) => (
                    <li key={i} className="mono">
                      {a.action} {a.skill} → {a.tool}: {a.error ?? '?'}
                    </li>
                  ))}
                </ul>
              ) : null}
            </div>
          )
        })}
      </div>

      <div className="fleet-onboard">
        <h2>{t('fleet.addMachine')}</h2>
        <p>{t('fleet.addMachineHelp')}</p>
        <pre className="fleet-snippet mono">{INSTALL_SNIPPET}</pre>
        <div className="fleet-onboard-actions">
          <button
            className="fleet-copy-btn"
            type="button"
            onClick={() => void copyText(INSTALL_SNIPPET, 'fleet.snippetCopied')}
          >
            <Copy size={14} /> {t('fleet.copySnippet')}
          </button>
          <button
            className="fleet-copy-btn"
            type="button"
            onClick={() => void copyText(AGENT_INSTRUCTIONS, 'fleet.instructionsCopied')}
          >
            <Copy size={14} /> {t('fleet.copyInstructions')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default FleetPage
