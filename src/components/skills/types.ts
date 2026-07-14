export type OnboardingVariant = {
  tool: string
  name: string
  path: string
  fingerprint?: string | null
  is_link: boolean
  link_target?: string | null
  plugin_name?: string | null
  plugin_version?: string | null
  plugin_scope?: string | null
}

export type OnboardingGroup = {
  name: string
  variants: OnboardingVariant[]
  has_conflict: boolean
}

export type OnboardingPlan = {
  total_tools_scanned: number
  total_skills_found: number
  groups: OnboardingGroup[]
}

export type ToolOption = {
  id: string
  label: string
  supports_project_scope?: boolean
}

export type TagDto = {
  id: number
  name: string
}

export type TagWithCountDto = TagDto & {
  skill_count: number
  updated_at: number
}

export type ManagedSkill = {
  id: string
  name: string
  description?: string | null
  source_type: string
  source_ref?: string | null
  central_path: string
  created_at: number
  updated_at: number
  last_sync_at?: number | null
  enabled: boolean
  status: string
  tags: TagDto[]
  targets: {
    tool: string
    scope: 'global' | 'project' | string
    project_path?: string | null
    mode: string
    status: string
    target_path: string
    synced_at?: number | null
  }[]
}

export type GitSkillCandidate = {
  name: string
  description?: string | null
  subpath: string
}

/** Everything the Explore detail modal knows before it lazy-loads the repo. */
export type ExploreDetailSeed = {
  name: string
  author: string
  summary: string
  license?: string | null
  sourceUrl: string
  origin: 'featured' | 'skills_sh' | 'github'
  count: number
  countIsStars: boolean
}

export type LocalSkillCandidate = {
  name: string
  description?: string | null
  subpath: string
  valid: boolean
  reason?: string | null
}

export type InstallResultDto = {
  skill_id: string
  name: string
  central_path: string
  content_hash?: string | null
}

export type ToolInfoDto = {
  key: string
  label: string
  /** Tool's directory exists on disk — independent of the user's toggle. */
  detected: boolean
  /** Detected AND enabled — eligibility as a sync target. */
  installed: boolean
  enabled: boolean
  is_custom: boolean
  adapter_kind: 'dir' | 'mcp'
  mcp_endpoint?: string
  skills_dir: string
  project_skills_dir: string
  supports_project_scope: boolean
}

export type ToolStatusDto = {
  tools: ToolInfoDto[]
  installed: string[]
  newly_installed: string[]
}

export type PinApplyResultDto = {
  skill: string
  tool: string
  ok: boolean
  error?: string
}

export type PinSyncResultDto = {
  pins: MachinePinsDto
  results: PinApplyResultDto[]
}

export type SkillVersionDto = {
  label: string
  addedAt: string
  isLatest: boolean
}

export type PinTargetDto = {
  enabled: boolean
  strategy: string
}

export type PinEntryDto = {
  skill: string
  version: string
  targets: Record<string, PinTargetDto>
}

export type MachinePinsDto = {
  machine: string
  pins: PinEntryDto[]
}

export type CustomToolConfigDto = {
  key: string
  label: string
  skills_dir: string
  project_skills_dir?: string | null
  enabled: boolean
}

export type ToolConfigDto = {
  disabled_builtin_tools: string[]
  custom_tools: CustomToolConfigDto[]
}

export type UpdateResultDto = {
  skill_id: string
  name: string
  content_hash?: string | null
  source_revision?: string | null
  updated_targets: string[]
}

export type AutoUpdateConfigDto = {
  enabled: boolean
  interval_hours: number
  schedule_type: 'interval' | 'daily'
  interval_value: number
  interval_unit: 'minutes' | 'hours'
  daily_time: string
  local_skill_count: number
  protected_local_skill_count: number
  task_registered: boolean
  task_status_detail: string
  last_run_at?: number | null
  last_started_at?: number | null
  last_finished_at?: number | null
  last_status?: string | null
  last_error?: string | null
  last_checked: number
  last_updated: number
  last_failed: number
  progress: AutoUpdateProgressSnapshotDto
}

export type AutoUpdateRunResultDto = {
  checked: number
  updated: number
  failed: number
  errors: string[]
  progress: AutoUpdateProgressSnapshotDto
}

export type GithubProxyConfigDto = {
  enabled: boolean
  port: number
  url: string
  auto_detected: boolean
}

export type AutoUpdateSkillProgressDto = {
  skill_id: string
  name: string
  reason?: string | null
}

export type AutoUpdateProgressSnapshotDto = {
  total: number
  succeeded: AutoUpdateSkillProgressDto[]
  failed: AutoUpdateSkillProgressDto[]
  running?: AutoUpdateSkillProgressDto | null
  pending: AutoUpdateSkillProgressDto[]
}

export type FeaturedSkillDto = {
  slug: string
  name: string
  summary: string
  downloads: number
  stars: number
  source_url: string
}

export type SkillOrigin = 'skills_sh' | 'github'

export type OnlineSkillDto = {
  name: string
  installs: number
  source: string
  source_url: string
  /** Absent when the index reports no identifiable license. Never inferred. */
  license?: string | null
  /** Which index answered; `github` means skills.sh was unreachable. */
  origin: SkillOrigin
}

export type SkillFileEntry = {
  path: string
  size: number
}
