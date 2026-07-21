import { memo, useEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { ArrowDown, ArrowUp, Check, CheckSquare, ChevronDown, Search, Tags } from 'lucide-react'
import type { TFunction } from 'i18next'
import type { TagWithCountDto } from './types'

type DropdownOption<T extends string> = { value: T; label: string }

/**
 * A themed single-select. Replaces a native <select>, whose OS-drawn option
 * list ignores the app theme (a stark white system popup in dark mode).
 */
function Dropdown<T extends string>({
  value,
  options,
  onChange,
  ariaLabel,
  icon,
}: {
  value: T
  options: DropdownOption<T>[]
  onChange: (value: T) => void
  ariaLabel: string
  icon: ReactNode
}) {
  const [open, setOpen] = useState(false)
  const ref = useRef<HTMLDivElement | null>(null)
  useEffect(() => {
    if (!open) return
    const onDown = (e: MouseEvent) => {
      if (!ref.current?.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('mousedown', onDown)
    return () => document.removeEventListener('mousedown', onDown)
  }, [open])
  const current = options.find((o) => o.value === value)
  return (
    <div className="fb-dropdown" ref={ref}>
      <button
        className="btn btn-secondary sort-btn"
        type="button"
        aria-label={ariaLabel}
        aria-haspopup="listbox"
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        {current?.label ?? ''}
        {icon}
        <ChevronDown size={12} />
      </button>
      {open ? (
        <div className="fb-dropdown-menu" role="listbox" aria-label={ariaLabel}>
          {options.map((o) => (
            <button
              key={o.value}
              type="button"
              role="option"
              aria-selected={o.value === value}
              className={`fb-dropdown-option${o.value === value ? ' selected' : ''}`}
              onClick={() => {
                onChange(o.value)
                setOpen(false)
              }}
            >
              <span className="fb-dropdown-check">
                {o.value === value ? <Check size={14} /> : null}
              </span>
              <span>{o.label}</span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  )
}

type FilterBarProps = {
  sortBy: 'updated' | 'name'
  sortDir: 'asc' | 'desc'
  searchQuery: string
  scopeFilter: 'all' | 'global' | 'project'
  tags: TagWithCountDto[]
  selectedTagIds: number[]
  includeUntagged: boolean
  untaggedCount: number
  totalCount: number
  bulkMode: boolean
  bulkSelectedCount: number
  onSortChange: (value: 'updated' | 'name') => void
  onSearchChange: (value: string) => void
  onScopeFilterChange: (value: 'all' | 'global' | 'project') => void
  onToggleTag: (tagId: number) => void
  onToggleUntagged: () => void
  onClearTags: () => void
  onManageTags: () => void
  onToggleBulkMode: () => void
  t: TFunction
}

const FilterBar = ({
  sortBy,
  sortDir,
  searchQuery,
  scopeFilter,
  tags,
  selectedTagIds,
  includeUntagged,
  untaggedCount,
  totalCount,
  bulkMode,
  bulkSelectedCount,
  onSortChange,
  onSearchChange,
  onScopeFilterChange,
  onToggleTag,
  onToggleUntagged,
  onClearTags,
  onManageTags,
  onToggleBulkMode,
  t,
}: FilterBarProps) => {
  const [tagMenuOpen, setTagMenuOpen] = useState(false)
  const [tagQuery, setTagQuery] = useState('')
  const tagMenuRef = useRef<HTMLDivElement | null>(null)
  const scopeOptions: { value: 'all' | 'global' | 'project'; label: string }[] = [
    { value: 'all', label: t('scope.all') },
    { value: 'global', label: t('scope.global') },
    { value: 'project', label: t('scope.project') },
  ]
  const selectedTagSet = useMemo(() => new Set(selectedTagIds), [selectedTagIds])
  const selectedCount = selectedTagIds.length + (includeUntagged ? 1 : 0)
  const filteredTags = useMemo(() => {
    const query = tagQuery.trim().toLowerCase()
    if (!query) return tags
    return tags.filter((tag) => tag.name.toLowerCase().includes(query))
  }, [tagQuery, tags])

  useEffect(() => {
    if (!tagMenuOpen) return
    const handlePointerDown = (event: MouseEvent) => {
      if (!tagMenuRef.current?.contains(event.target as Node)) {
        setTagMenuOpen(false)
      }
    }
    document.addEventListener('mousedown', handlePointerDown)
    return () => document.removeEventListener('mousedown', handlePointerDown)
  }, [tagMenuOpen])

  return (
    <div className="filter-bar">
      <div className="filter-title">
        {t('allSkills')}（{totalCount}）
      </div>
      <div className="filter-actions">
        <Dropdown
          value={scopeFilter}
          options={scopeOptions}
          onChange={onScopeFilterChange}
          ariaLabel={t('scope.filterLabel')}
          icon={null}
        />
        <Dropdown
          value={sortBy}
          options={[
            { value: 'updated', label: t('sortUpdated') },
            { value: 'name', label: t('sortName') },
          ]}
          onChange={onSortChange}
          ariaLabel={t('filterSort')}
          // The arrow shows the active direction; re-picking the sort flips it.
          icon={sortDir === 'asc' ? <ArrowUp size={12} /> : <ArrowDown size={12} />}
        />
        <button
          className={`btn btn-secondary bulk-mode-btn${bulkMode ? ' active' : ''}`}
          type="button"
          onClick={onToggleBulkMode}
        >
          <CheckSquare size={14} />
          {bulkMode
            ? t('bulk.selectedShort', { count: bulkSelectedCount })
            : t('bulk.manage')}
        </button>
        <div className="tag-filter-wrap" ref={tagMenuRef}>
          <button
            className={`btn btn-secondary tag-filter-btn${selectedCount > 0 ? ' active' : ''}`}
            type="button"
            onClick={() => setTagMenuOpen((open) => !open)}
          >
            <Tags size={14} />
            {selectedCount > 0
              ? t('tagsSelected', { count: selectedCount })
              : t('tags')}
            <ChevronDown size={12} />
          </button>
          {tagMenuOpen ? (
            <div className="tag-filter-menu">
              <div className="tag-filter-head">
                <span>{t('tags')}</span>
                <span>{t('matchAny')}</span>
              </div>
              <div className="tag-filter-search">
                <Search size={15} />
                <input
                  value={tagQuery}
                  onChange={(event) => setTagQuery(event.target.value)}
                  placeholder={t('searchTags')}
                />
              </div>
              <div className="tag-filter-options">
                <button
                  className={`tag-filter-option${includeUntagged ? ' selected' : ''}`}
                  type="button"
                  onClick={onToggleUntagged}
                >
                  <span className="tag-check">{includeUntagged ? <Check size={14} /> : null}</span>
                  <span>{t('untagged')}</span>
                  <span className="tag-count">{untaggedCount}</span>
                </button>
                {filteredTags.map((tag) => {
                  const selected = selectedTagSet.has(tag.id)
                  return (
                    <button
                      key={tag.id}
                      className={`tag-filter-option${selected ? ' selected' : ''}`}
                      type="button"
                      onClick={() => onToggleTag(tag.id)}
                    >
                      <span className="tag-check">{selected ? <Check size={14} /> : null}</span>
                      <span>{tag.name}</span>
                      <span className="tag-count">{tag.skill_count}</span>
                    </button>
                  )
                })}
              </div>
              <div className="tag-filter-footer">
                <button type="button" onClick={onClearTags} disabled={selectedCount === 0}>
                  {t('clearAll')}
                </button>
                <button type="button" onClick={onManageTags}>
                  {t('manageTags')}
                </button>
              </div>
            </div>
          ) : null}
        </div>
        <div className="search-container">
          <Search size={16} className="search-icon-abs" />
          <input
            className="search-input"
            value={searchQuery}
            onChange={(event) => onSearchChange(event.target.value)}
            placeholder={t('searchPlaceholder')}
          />
        </div>
      </div>
    </div>
  )
}

export default memo(FilterBar)
