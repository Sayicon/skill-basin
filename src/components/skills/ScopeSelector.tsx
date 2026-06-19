import { memo, useId, useMemo } from 'react'
import { Folder, X } from 'lucide-react'
import type { TFunction } from 'i18next'
import {
  getAvailableRecentProjects,
  normalizeProjectPaths,
  type InstallScope,
} from './installScope'

type ScopeSelectorProps = {
  scope: InstallScope
  projects: string[]
  recentProjects: string[]
  disabled?: boolean
  showRequired?: boolean
  onScopeChange: (scope: InstallScope) => void
  onProjectsChange: (projects: string[]) => void
  onPickProject: () => Promise<string | undefined>
  t: TFunction
}

const ScopeSelector = ({
  scope,
  projects,
  recentProjects,
  disabled = false,
  showRequired = true,
  onScopeChange,
  onProjectsChange,
  onPickProject,
  t,
}: ScopeSelectorProps) => {
  const scopeRadioName = useId()
  const normalizedProjects = useMemo(
    () => normalizeProjectPaths(projects),
    [projects],
  )
  const availableRecentProjects = useMemo(
    () => getAvailableRecentProjects(recentProjects, normalizedProjects),
    [recentProjects, normalizedProjects],
  )

  const addProject = (projectPath: string) => {
    onProjectsChange(normalizeProjectPaths([...normalizedProjects, projectPath]))
  }

  return (
    <div className="scope-selector">
      <label className={`scope-choice${scope === 'global' ? ' active' : ''}`}>
        <input
          type="radio"
          name={scopeRadioName}
          checked={scope === 'global'}
          onChange={() => onScopeChange('global')}
          disabled={disabled}
        />
        <span>
          <strong>{t('scope.global')}</strong>
          <small>{t('projectSync.globalDesc')}</small>
        </span>
      </label>
      <label className={`scope-choice${scope === 'project' ? ' active' : ''}`}>
        <input
          type="radio"
          name={scopeRadioName}
          checked={scope === 'project'}
          onChange={() => onScopeChange('project')}
          disabled={disabled}
        />
        <span>
          <strong>{t('scope.project')}</strong>
          <small>{t('projectSync.projectDesc')}</small>
        </span>
      </label>

      {scope === 'project' ? (
        <div className="project-sync-panel">
          <div className="project-sync-heading">{t('projectSync.projectDirs')}</div>
          {normalizedProjects.length > 0 ? (
            <div className="project-path-list">
              {normalizedProjects.map((project) => (
                <div className="project-path-row" key={project}>
                  <Folder size={14} />
                  <span className="mono">{project}</span>
                  <button
                    type="button"
                    className="icon-btn"
                    onClick={() =>
                      onProjectsChange(
                        normalizedProjects.filter((item) => item !== project),
                      )
                    }
                    disabled={disabled}
                    aria-label={t('remove')}
                  >
                    <X size={14} />
                  </button>
                </div>
              ))}
            </div>
          ) : (
            <div className="project-empty">{t('projectSync.noProjects')}</div>
          )}
          {showRequired && normalizedProjects.length === 0 ? (
            <div className="scope-inline-warning">
              {t('projectSync.projectRequired')}
            </div>
          ) : null}
          <button
            type="button"
            className="btn btn-secondary"
            onClick={() => {
              void onPickProject().then((projectPath) => {
                if (projectPath) addProject(projectPath)
              })
            }}
            disabled={disabled}
          >
            {t('projectSync.addProject')}
          </button>

          {availableRecentProjects.length > 0 ? (
            <>
              <div className="project-sync-heading">
                {t('projectSync.recentProjects')}
              </div>
              <div className="recent-project-list">
                {availableRecentProjects.map((project) => (
                  <button
                    key={project}
                    type="button"
                    className="recent-project-row"
                    onClick={() => addProject(project)}
                    disabled={disabled}
                  >
                    <span className="mono">{project}</span>
                    <span>{t('projectSync.addRecent')}</span>
                  </button>
                ))}
              </div>
            </>
          ) : null}
        </div>
      ) : null}
    </div>
  )
}

export default memo(ScopeSelector)
