import { memo, useEffect, useState } from 'react'
import {
  ExternalLink,
  Github,
  Package,
  Scale,
  ShieldAlert,
  Star,
} from 'lucide-react'
import type { TFunction } from 'i18next'
import type { ExploreDetailSeed, GitSkillCandidate } from '../types'

type ExploreDetailModalProps = {
  open: boolean
  loading: boolean
  seed: ExploreDetailSeed | null
  fetchSubSkills: (repoUrl: string) => Promise<GitSkillCandidate[]>
  onInstall: (seed: ExploreDetailSeed) => void
  onOpenExternal: (url: string) => void
  onRequestClose: () => void
  t: TFunction
}

function formatCount(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`
  return String(n)
}

const ORIGIN_LABEL: Record<ExploreDetailSeed['origin'], string> = {
  featured: 'explore.sourceFeatured',
  skills_sh: 'explore.sourceSkillsSh',
  github: 'explore.sourceGithub',
}

const ExploreDetailModal = ({
  open,
  loading,
  seed,
  fetchSubSkills,
  onInstall,
  onOpenExternal,
  onRequestClose,
  t,
}: ExploreDetailModalProps) => {
  const [subSkills, setSubSkills] = useState<GitSkillCandidate[]>([])
  const [subState, setSubState] = useState<'idle' | 'loading' | 'error' | 'done'>('idle')

  const sourceUrl = seed?.sourceUrl ?? ''

  // Flip to the loading state synchronously in render (guarded so it runs once
  // per source) rather than in the effect below, which would paint a stale
  // frame first and trip the set-state-in-effect lint.
  const [loadedUrl, setLoadedUrl] = useState('')
  if (open && sourceUrl && loadedUrl !== sourceUrl) {
    setLoadedUrl(sourceUrl)
    setSubState('loading')
    setSubSkills([])
  }

  // Lazy-load the repo's skill structure once the modal opens for a given
  // source. Guard against a stale response landing after the user has moved on
  // to a different card (or closed the modal); setState only in the async
  // callbacks keeps the effect lint-clean.
  useEffect(() => {
    if (!open || !sourceUrl) return
    let active = true
    fetchSubSkills(sourceUrl)
      .then((skills) => {
        if (!active) return
        setSubSkills(skills)
        setSubState('done')
      })
      .catch(() => {
        if (!active) return
        setSubState('error')
      })
    return () => {
      active = false
    }
  }, [open, sourceUrl, fetchSubSkills])

  if (!open || !seed) return null

  const isMulti = subState === 'done' && subSkills.length > 1

  return (
    <div className="modal-backdrop" onClick={onRequestClose}>
      <div
        className="modal explore-detail-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="modal-header">
          <div className="explore-detail-heading">
            <div className="modal-title">{seed.name}</div>
            <div className="explore-detail-author">
              <Github size={13} />
              {seed.author}
            </div>
          </div>
          <button
            className="modal-close"
            type="button"
            onClick={onRequestClose}
            aria-label={t('close')}
          >
            ✕
          </button>
        </div>

        <div className="modal-body">
          <div className="explore-detail-meta">
            <span className="explore-detail-chip">
              {seed.countIsStars ? <Star size={13} /> : <Package size={13} />}
              {seed.countIsStars
                ? formatCount(seed.count)
                : t('explore.installCount', {
                    count: seed.count,
                    formatted: formatCount(seed.count),
                  })}
            </span>
            <span className="explore-detail-chip">
              {t(ORIGIN_LABEL[seed.origin])}
            </span>
            {seed.origin !== 'featured' ? (
              seed.license ? (
                <span className="explore-detail-chip">
                  <Scale size={13} />
                  {seed.license}
                </span>
              ) : (
                <span className="explore-detail-chip warn">
                  <ShieldAlert size={13} />
                  {t('explore.licenseMissing')}
                </span>
              )
            ) : null}
          </div>

          {seed.summary ? (
            <p className="explore-detail-summary">{seed.summary}</p>
          ) : null}

          <button
            className="explore-detail-source"
            type="button"
            onClick={() => onOpenExternal(sourceUrl)}
            title={sourceUrl}
          >
            <span className="mono">{sourceUrl.replace('https://', '')}</span>
            <ExternalLink size={13} />
          </button>

          <div className="explore-detail-section">
            <div className="explore-detail-section-title">
              {t('exploreDetail.contents')}
              {subState === 'done' ? (
                <span className="explore-detail-count mono">{subSkills.length}</span>
              ) : null}
            </div>
            {subState === 'loading' ? (
              <div className="explore-detail-sub-note">
                {t('exploreDetail.loadingContents')}
              </div>
            ) : subState === 'error' ? (
              <div className="explore-detail-sub-note error">
                {t('exploreDetail.contentsError')}
              </div>
            ) : subSkills.length === 0 ? (
              <div className="explore-detail-sub-note">
                {t('exploreDetail.contentsEmpty')}
              </div>
            ) : (
              <ul className="explore-detail-sub-list">
                {subSkills.map((sub) => (
                  <li className="explore-detail-sub" key={sub.subpath}>
                    <div className="explore-detail-sub-name">{sub.name}</div>
                    {sub.description ? (
                      <div className="explore-detail-sub-desc">{sub.description}</div>
                    ) : null}
                    <div className="explore-detail-sub-path mono">{sub.subpath}</div>
                  </li>
                ))}
              </ul>
            )}
          </div>
        </div>

        <div className="modal-footer">
          <button
            className="btn btn-secondary"
            type="button"
            onClick={() => onOpenExternal(sourceUrl)}
          >
            <Github size={14} />
            {t('exploreDetail.openSource')}
          </button>
          <button
            className="btn btn-primary"
            type="button"
            onClick={() => onInstall(seed)}
            disabled={loading}
          >
            {isMulti ? t('exploreDetail.installPick') : t('install')}
          </button>
        </div>
      </div>
    </div>
  )
}

export default memo(ExploreDetailModal)
