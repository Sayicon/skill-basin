import type { SetStateAction } from 'react'
import type { ToolOption } from './types'

export type InstallScope = 'global' | 'project'

export type InstallSyncJob =
  | { toolId: string; scope: 'global' }
  | { toolId: string; scope: 'project'; projectPath: string }

export const normalizeProjectPaths = (projects: string[]): string[] =>
  Array.from(
    new Set(projects.map((project) => project.trim()).filter(Boolean)),
  )

export const getAvailableRecentProjects = (
  recentProjects: string[],
  selectedProjects: string[],
): string[] => {
  const selected = new Set(normalizeProjectPaths(selectedProjects))

  return normalizeProjectPaths(recentProjects).filter(
    (project) => !selected.has(project),
  )
}

export const getAddedProjectPaths = (
  previousProjects: string[],
  nextProjects: string[],
): string[] => {
  const previous = new Set(normalizeProjectPaths(previousProjects))

  return normalizeProjectPaths(nextProjects).filter(
    (project) => !previous.has(project),
  )
}

export const resolveProjectPathsUpdate = (
  currentProjects: string[],
  update: SetStateAction<string[]>,
): string[] =>
  normalizeProjectPaths(
    typeof update === 'function' ? update(currentProjects) : update,
  )

export const isLatestSaveBatch = (
  batchSequence: number,
  latestSequence: number,
): boolean => batchSequence === latestSequence

export const isToolUnsupportedForScope = (
  tool: ToolOption,
  scope: InstallScope,
): boolean =>
  scope === 'project' && tool.supports_project_scope === false

export const getUnsupportedToolsForScope = (
  tools: ToolOption[],
  scope: InstallScope,
): ToolOption[] =>
  tools.filter((tool) => isToolUnsupportedForScope(tool, scope))

export const filterTargetsForScope = (
  targets: Record<string, boolean>,
  tools: ToolOption[],
  scope: InstallScope,
): Record<string, boolean> => {
  if (scope === 'global') return { ...targets }

  const supportsProject = new Map(
    tools.map((tool) => [tool.id, tool.supports_project_scope ?? true]),
  )

  return Object.fromEntries(
    Object.entries(targets).map(([toolId, selected]) => [
      toolId,
      selected && (supportsProject.get(toolId) ?? true),
    ]),
  )
}

export const buildInstallSyncJobs = (
  toolIds: string[],
  scope: InstallScope,
  projects: string[],
): InstallSyncJob[] => {
  if (scope === 'global') {
    return toolIds.map((toolId) => ({ toolId, scope: 'global' }))
  }

  const projectPaths = normalizeProjectPaths(projects)

  return toolIds.flatMap((toolId) =>
    projectPaths.map((projectPath) => ({
      toolId,
      scope: 'project' as const,
      projectPath,
    })),
  )
}
