import { memo, useMemo, useState } from 'react'
import { Plus, Scale, Search, ShieldAlert, Star } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { FeaturedSkillDto, ManagedSkill, OnlineSkillDto } from './types'

type ExplorePageProps = {
  featuredSkills: FeaturedSkillDto[]
  featuredLoading: boolean
  exploreFilter: string
  searchResults: OnlineSkillDto[]
  searchLoading: boolean
  managedSkills: ManagedSkill[]
  loading: boolean
  onExploreFilterChange: (value: string) => void
  onInstallSkill: (
    sourceUrl: string,
    skillName?: string,
    /** Undefined means the source declared no license — the modal warns. */
    license?: string | null,
  ) => void
  onOpenManualAdd: () => void
  t: TFunction
}

/** Which index a result may come from; `featured` is the curated bundle. */
type SourceFilter = 'all' | 'featured' | 'skills_sh' | 'github'

function formatCount(n: number): string {
  if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`
  return String(n)
}

/**
 * A skill with no identifiable license grants no rights. Say so plainly
 * rather than leaving the field blank, which reads as "probably fine".
 */
const LicenseTag = ({ license, t }: { license?: string | null; t: TFunction }) =>
  license ? (
    <span className="explore-license" title={t('explore.licenseKnown', { license })}>
      <Scale size={11} />
      {license}
    </span>
  ) : (
    <span className="explore-license missing" title={t('explore.licenseMissingHint')}>
      <ShieldAlert size={11} />
      {t('explore.licenseMissing')}
    </span>
  )

const ExplorePage = ({
  featuredSkills,
  featuredLoading,
  exploreFilter,
  searchResults,
  searchLoading,
  managedSkills,
  loading,
  onExploreFilterChange,
  onInstallSkill,
  onOpenManualAdd,
  t,
}: ExplorePageProps) => {
  // Some feed entries carry a bare YAML block-scalar indicator ("|-", ">")
  // instead of a real summary — the generator failed on a multi-line
  // description. Render those as empty rather than leaking the token.
  const cleanSummary = (summary: string) => {
    const trimmed = summary.trim()
    return /^[|>][+-]?$/.test(trimmed) ? '' : trimmed
  }

  const filteredSkills = useMemo(() => {
    if (!exploreFilter.trim()) return featuredSkills
    const lower = exploreFilter.toLowerCase()
    return featuredSkills.filter(
      (s) =>
        s.name.toLowerCase().includes(lower) ||
        s.summary.toLowerCase().includes(lower),
    )
  }, [featuredSkills, exploreFilter])

  const [sourceFilter, setSourceFilter] = useState<SourceFilter>('all')

  const deduplicatedResults = useMemo(() => {
    const featuredNames = new Set(filteredSkills.map((s) => s.name.toLowerCase()))
    return searchResults.filter((s) => !featuredNames.has(s.name.toLowerCase()))
  }, [searchResults, filteredSkills])

  const visibleOnlineResults = useMemo(
    () =>
      sourceFilter === 'all'
        ? deduplicatedResults
        : deduplicatedResults.filter((skill) => skill.origin === sourceFilter),
    [deduplicatedResults, sourceFilter],
  )

  const isSearchActive = exploreFilter.trim().length >= 2
  const showFeatured = sourceFilter === 'all' || sourceFilter === 'featured'
  const showOnline = sourceFilter !== 'featured'

  // Results silently switch index when skills.sh is unreachable; the user
  // should know they are looking at a fallback, not the usual catalogue.
  const usingFallback =
    deduplicatedResults.length > 0 &&
    deduplicatedResults.every((skill) => skill.origin === 'github')

  const sourceFilters: { id: SourceFilter; label: string; count?: number }[] = [
    { id: 'all', label: t('explore.sourceAll') },
    { id: 'featured', label: t('explore.sourceFeatured'), count: filteredSkills.length },
    {
      id: 'skills_sh',
      label: t('explore.sourceSkillsSh'),
      count: deduplicatedResults.filter((s) => s.origin === 'skills_sh').length,
    },
    {
      id: 'github',
      label: t('explore.sourceGithub'),
      count: deduplicatedResults.filter((s) => s.origin === 'github').length,
    },
  ]

  // Check if a skill is already installed by matching name + source (case-insensitive)
  const installedSkillKeys = useMemo(() => {
    const keys = new Set<string>()
    for (const skill of managedSkills) {
      const source = (skill.source_ref ?? '')
        .replace('https://github.com/', '')
        .replace(/\.git$/, '')
        .split('/tree/')[0]
        .toLowerCase()
      keys.add(`${skill.name.toLowerCase()}|${source}`)
    }
    return keys
  }, [managedSkills])

  const isInstalled = (skillName: string, source: string) => {
    const normalizedSource = source
      .replace('https://github.com/', '')
      .replace(/\.git$/, '')
      .split('/tree/')[0]
      .toLowerCase()
    return installedSkillKeys.has(`${skillName.toLowerCase()}|${normalizedSource}`)
  }

  return (
    <div className="explore-page">
      <div className="explore-hero">
        <div className="explore-search-row">
          <div className="explore-search-wrap">
            <Search size={16} className="explore-search-icon" />
            <input
              className="explore-search-input"
              placeholder={t('exploreFilterPlaceholder')}
              value={exploreFilter}
              onChange={(e) => onExploreFilterChange(e.target.value)}
            />
          </div>
          <button
            className="btn btn-secondary explore-manual-btn"
            type="button"
            onClick={onOpenManualAdd}
            disabled={loading}
          >
            <Plus size={15} />
            {t('manualAdd')}
          </button>
        </div>
        <div className="explore-source-label">
          {t('exploreSourceHint')}
        </div>
        <div className="explore-source-filters" role="group" aria-label={t('explore.sourceFilter')}>
          {sourceFilters.map((filter) => (
            <button
              key={filter.id}
              type="button"
              className={`explore-source-chip${sourceFilter === filter.id ? ' active' : ''}`}
              aria-pressed={sourceFilter === filter.id}
              onClick={() => setSourceFilter(filter.id)}
            >
              {filter.label}
              {filter.count !== undefined && filter.count > 0 ? (
                <span className="explore-source-count mono">{filter.count}</span>
              ) : null}
            </button>
          ))}
        </div>
        {usingFallback ? (
          <div className="explore-fallback-note">
            <ShieldAlert size={13} />
            {t('explore.fallbackNote')}
          </div>
        ) : null}
      </div>

      <div className="explore-scroll">
        {/* Featured section */}
        {featuredLoading ? (
          <div className="explore-loading">{t('exploreLoading')}</div>
        ) : (
          <>
            {showFeatured && isSearchActive && filteredSkills.length > 0 && (
              <div className="explore-section-title">{t('exploreFeaturedTitle')}</div>
            )}
            {showFeatured && filteredSkills.length > 0 ? (
              <div className="explore-grid">
                {filteredSkills.map((skill) => {
                  const installed = isInstalled(skill.name, skill.source_url)
                  return (
                    <div key={skill.slug} className="explore-card">
                      <div className="explore-card-top">
                        <div className="explore-card-info">
                          <div className="explore-card-name">{skill.name}</div>
                          <div className="explore-card-author">
                            {skill.source_url
                              .replace('https://github.com/', '')
                              .split('/tree/')[0]}
                          </div>
                        </div>
                        {installed ? (
                          <span className="explore-btn-installed">
                            {t('status.installed')}
                          </span>
                        ) : (
                          <button
                            className="explore-btn-install"
                            type="button"
                            disabled={loading}
                            onClick={() => onInstallSkill(skill.source_url)}
                          >
                            {t('install')}
                          </button>
                        )}
                      </div>
                      <div className="explore-card-desc">{cleanSummary(skill.summary)}</div>
                      <div className="explore-card-bottom">
                        <div className="explore-card-stats">
                          <span className="explore-stat">
                            <Star size={12} />
                            {formatCount(skill.stars)}
                          </span>
                        </div>
                      </div>
                    </div>
                  )
                })}
              </div>
            ) : !isSearchActive && showFeatured ? (
              <div className="explore-empty">{t('exploreEmpty')}</div>
            ) : null}

            {/* Online search results */}
            {isSearchActive && showOnline && (
              <>
                <div className="explore-section-title">{t('exploreOnlineTitle')}</div>
                {searchLoading ? (
                  <div className="explore-loading">{t('searchLoading')}</div>
                ) : visibleOnlineResults.length > 0 ? (
                  <div className="explore-grid">
                    {visibleOnlineResults.map((skill) => {
                      const installed = isInstalled(skill.name, skill.source_url)
                      return (
                        <div key={skill.source} className="explore-card">
                          <div className="explore-card-top">
                            <div className="explore-card-info">
                              <div className="explore-card-name">{skill.name}</div>
                              <div className="explore-card-author">{skill.source}</div>
                            </div>
                            {installed ? (
                              <span className="explore-btn-installed">
                                {t('status.installed')}
                              </span>
                            ) : (
                              <button
                                className="explore-btn-install"
                                type="button"
                                disabled={loading}
                                onClick={() =>
                                  onInstallSkill(skill.source_url, skill.name, skill.license)
                                }
                              >
                                {t('install')}
                              </button>
                            )}
                          </div>
                          <div className="explore-card-bottom">
                            <div className="explore-card-stats">
                              <span className="explore-stat">
                                {skill.origin === 'github' ? (
                                  <>
                                    <Star size={12} />
                                    {formatCount(skill.installs)}
                                  </>
                                ) : (
                                  t('explore.installCount', {
                                    count: skill.installs,
                                    formatted: formatCount(skill.installs),
                                  })
                                )}
                              </span>
                              <LicenseTag license={skill.license} t={t} />
                            </div>
                          </div>
                        </div>
                      )
                    })}
                  </div>
                ) : (
                  <div className="explore-empty">{t('searchEmpty')}</div>
                )}
              </>
            )}
          </>
        )}
      </div>
    </div>
  )
}

export default memo(ExplorePage)
