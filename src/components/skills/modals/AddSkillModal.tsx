import { memo, type SetStateAction } from 'react'
import { Check, ShieldAlert } from 'lucide-react'
import type { TFunction } from 'i18next'
import ScopeSelector from '../ScopeSelector'
import {
  getUnsupportedToolsForScope,
  isToolUnsupportedForScope,
  normalizeProjectPaths,
  type InstallScope,
} from '../installScope'
import type { TagWithCountDto, ToolOption, ToolStatusDto } from '../types'

type AddSkillModalProps = {
  open: boolean
  loading: boolean
  canClose: boolean
  /** The chosen source declares no license — say so before anything is copied. */
  licenseWarning?: boolean
  addModalTab: 'local' | 'git'
  localPath: string
  localName: string
  gitUrl: string
  gitName: string
  tags: TagWithCountDto[]
  selectedTagIds: number[]
  syncTargets: Record<string, boolean>
  installedTools: ToolOption[]
  toolStatus: ToolStatusDto | null
  installScope: InstallScope
  installProjects: string[]
  recentProjects: string[]
  onRequestClose: () => void
  onTabChange: (tab: 'local' | 'git') => void
  onLocalPathChange: (value: string) => void
  onPickLocalPath: () => void
  onLocalNameChange: (value: string) => void
  onGitUrlChange: (value: string) => void
  onGitNameChange: (value: string) => void
  onToggleTag: (tagId: number) => void
  onSyncTargetChange: (toolId: string, checked: boolean) => void
  onInstallScopeChange: (scope: InstallScope) => void
  onInstallProjectsChange: (projects: SetStateAction<string[]>) => void
  onPickProject: () => Promise<string | undefined>
  onSubmit: () => void
  t: TFunction
}

const AddSkillModal = ({
  open,
  loading,
  canClose,
  licenseWarning = false,
  addModalTab,
  localPath,
  localName,
  gitUrl,
  gitName,
  tags,
  selectedTagIds,
  syncTargets,
  installedTools,
  toolStatus,
  installScope,
  installProjects,
  recentProjects,
  onRequestClose,
  onTabChange,
  onLocalPathChange,
  onPickLocalPath,
  onLocalNameChange,
  onGitUrlChange,
  onGitNameChange,
  onToggleTag,
  onSyncTargetChange,
  onInstallScopeChange,
  onInstallProjectsChange,
  onPickProject,
  onSubmit,
  t,
}: AddSkillModalProps) => {
  if (!open) return null

  const projectRequired =
    installScope === 'project' &&
    normalizeProjectPaths(installProjects).length === 0
  const unsupportedTools = getUnsupportedToolsForScope(
    installedTools,
    installScope,
  )

  return (
    <div
      className="modal-backdrop"
      onClick={() => (canClose ? onRequestClose() : null)}
    >
      <div className="modal add-skill-modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-header">
          <div className="modal-title">{t('addSkillTitle')}</div>
          <button
            className="modal-close"
            type="button"
            onClick={onRequestClose}
            aria-label={t('close')}
            disabled={!canClose}
          >
            ✕
          </button>
        </div>
        <div className="modal-body">
          {licenseWarning ? (
            <div className="modal-license-warning" role="alert">
              <ShieldAlert size={15} />
              <span>{t('explore.licenseMissingHint')}</span>
            </div>
          ) : null}
          <div className="tabs">
            <button
              className={`tab-item${addModalTab === 'local' ? ' active' : ''}`}
              type="button"
              onClick={() => onTabChange('local')}
            >
              {t('localTab')}
            </button>
            <button
              className={`tab-item${addModalTab === 'git' ? ' active' : ''}`}
              type="button"
              onClick={() => onTabChange('git')}
            >
              {t('gitTab')}
            </button>
          </div>

          {addModalTab === 'local' ? (
            <>
              <div className="form-group form-row-inline">
                <div className="form-field">
                  <label className="label">{t('localFolder')}</label>
                  <div className="input-row">
                    <input
                      className="input"
                      placeholder={t('localPathPlaceholder')}
                      value={localPath}
                      onChange={(event) => onLocalPathChange(event.target.value)}
                    />
                    <button
                      className="btn btn-secondary input-action"
                      type="button"
                      onClick={onPickLocalPath}
                      disabled={!canClose}
                    >
                      {t('browse')}
                    </button>
                  </div>
                </div>
                <div className="form-field">
                  <label className="label">{t('optionalNamePlaceholder')}</label>
                  <input
                    className="input"
                    placeholder={t('optionalNamePlaceholder')}
                    value={localName}
                    onChange={(event) => onLocalNameChange(event.target.value)}
                  />
                </div>
              </div>
            </>
          ) : (
            <>
              <div className="form-group form-row-inline">
                <div className="form-field">
                  <label className="label">{t('repositoryUrl')}</label>
                  <input
                    className="input"
                    placeholder={t('gitUrlPlaceholder')}
                    value={gitUrl}
                    onChange={(event) => onGitUrlChange(event.target.value)}
                  />
                </div>
                <div className="form-field">
                  <label className="label">{t('optionalNamePlaceholder')}</label>
                  <input
                    className="input"
                    placeholder={t('optionalNamePlaceholder')}
                    value={gitName}
                    onChange={(event) => onGitNameChange(event.target.value)}
                  />
                </div>
              </div>
            </>
          )}

          <div className="form-group tag-field-inline">
            <label className="label">{t('addTags')}</label>
            {tags.length > 0 ? (
              <div className="add-tags-list">
                {tags.map((tag) => {
                  const selected = selectedTagIds.includes(tag.id)
                  return (
                    <button
                      key={tag.id}
                      className={`add-tag-pill${selected ? ' selected' : ''}`}
                      type="button"
                      onClick={() => onToggleTag(tag.id)}
                    >
                      <span className="add-tag-check">
                        {selected ? <Check size={12} /> : null}
                      </span>
                      <span>#{tag.name}</span>
                    </button>
                  )
                })}
              </div>
            ) : (
              <div className="helper-text">{t('noTagsYet')}</div>
            )}
          </div>

          <div className="form-group">
            <ScopeSelector
              scope={installScope}
              projects={installProjects}
              recentProjects={recentProjects}
              disabled={loading}
              compact
              title={t('installScope.title')}
              onScopeChange={onInstallScopeChange}
              onProjectsChange={onInstallProjectsChange}
              onPickProject={onPickProject}
              t={t}
            />
          </div>

          <div className="form-group">
            <label className="label">{t('installToTools')}</label>
            {toolStatus ? (
              <div className="tool-matrix">
                {installedTools.map((tool) => {
                  const unsupported = isToolUnsupportedForScope(
                    tool,
                    installScope,
                  )

                  return (
                    <label
                      key={tool.id}
                      className={`tool-pill-toggle${
                        syncTargets[tool.id] ? ' active' : ''
                      }${unsupported ? ' disabled' : ''}`}
                      title={
                        unsupported
                          ? t('installScope.unsupportedTool', {
                              tool: tool.label,
                            })
                          : undefined
                      }
                    >
                      <input
                        type="checkbox"
                        checked={Boolean(syncTargets[tool.id])}
                        onChange={(event) =>
                          onSyncTargetChange(tool.id, event.target.checked)
                        }
                        disabled={unsupported}
                      />
                      {syncTargets[tool.id] ? (
                        <span className="status-badge" />
                      ) : null}
                      {tool.label}
                    </label>
                  )
                })}
              </div>
            ) : (
              <div className="helper-text">{t('detectingTools')}</div>
            )}
            {unsupportedTools.length > 0 ? (
              <div className="helper-text" role="status">
                {t('installScope.unsupportedSelectedHint', {
                  tools: unsupportedTools.map((tool) => tool.label).join(', '),
                })}
              </div>
            ) : null}
          </div>
        </div>
        <div className="modal-footer">
          <button
            className="btn btn-secondary"
            onClick={onRequestClose}
            disabled={!canClose}
          >
            {t('cancel')}
          </button>
          <button
            className="btn btn-primary"
            onClick={onSubmit}
            disabled={loading || projectRequired}
          >
            {addModalTab === 'local' ? t('create') : t('install')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default memo(AddSkillModal)
