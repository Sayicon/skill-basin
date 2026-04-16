import { memo } from 'react'
import { ArrowUpDown, ChevronDown, RefreshCw, Search } from 'lucide-react'
import type { TFunction } from 'i18next'

type FilterBarProps = {
  sortBy: 'updated' | 'name'
  searchQuery: string
  scopeFilter: 'all' | 'global' | 'project'
  totalCount: number
  loading: boolean
  onSortChange: (value: 'updated' | 'name') => void
  onSearchChange: (value: string) => void
  onScopeFilterChange: (value: 'all' | 'global' | 'project') => void
  onRefresh: () => void
  t: TFunction
}

const FilterBar = ({
  sortBy,
  searchQuery,
  scopeFilter,
  totalCount,
  loading,
  onSortChange,
  onSearchChange,
  onScopeFilterChange,
  onRefresh,
  t,
}: FilterBarProps) => {
  const scopeOptions: { value: 'all' | 'global' | 'project'; label: string }[] = [
    { value: 'all', label: t('scope.all') },
    { value: 'global', label: t('scope.global') },
    { value: 'project', label: t('scope.project') },
  ]

  return (
    <div className="filter-bar">
      <div className="filter-title">
        {t('allSkills')}（{totalCount}）
      </div>
      <div className="filter-actions">
        <button className="btn btn-secondary sort-btn" type="button">
          {scopeOptions.find((option) => option.value === scopeFilter)?.label ?? t('scope.all')}
          <ChevronDown size={12} />
          <select
            aria-label={t('scope.filterLabel')}
            value={scopeFilter}
            onChange={(event) =>
              onScopeFilterChange(event.target.value as 'all' | 'global' | 'project')
            }
          >
            {scopeOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
        </button>
        <button className="btn btn-secondary sort-btn" type="button">
          {sortBy === 'updated' ? t('sortUpdated') : t('sortName')}
          <ArrowUpDown size={12} />
          <select
            aria-label={t('filterSort')}
            value={sortBy}
            onChange={(event) => onSortChange(event.target.value as 'updated' | 'name')}
          >
            <option value="updated">{t('sortUpdated')}</option>
            <option value="name">{t('sortName')}</option>
          </select>
        </button>
        <div className="search-container">
          <Search size={16} className="search-icon-abs" />
          <input
            className="search-input"
            value={searchQuery}
            onChange={(event) => onSearchChange(event.target.value)}
            placeholder={t('searchPlaceholder')}
          />
        </div>
        <button
          className="btn btn-secondary"
          type="button"
          onClick={onRefresh}
          disabled={loading}
        >
          <RefreshCw size={14} />
          {t('refresh')}
        </button>
      </div>
    </div>
  )
}

export default memo(FilterBar)
