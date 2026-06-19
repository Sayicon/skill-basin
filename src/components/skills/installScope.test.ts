import { describe, expect, it } from 'vitest'
import {
  buildInstallSyncJobs,
  filterTargetsForScope,
  normalizeProjectPaths,
} from './installScope'
import type { ToolOption } from './types'

const tools: ToolOption[] = [
  { id: 'cursor', label: 'Cursor', supports_project_scope: true },
  { id: 'hermes', label: 'Hermes Agent', supports_project_scope: false },
  { id: 'codex', label: 'Codex' },
]

describe('normalizeProjectPaths', () => {
  it('removes empty and duplicate project paths', () => {
    expect(
      normalizeProjectPaths([
        '/repo/a',
        '',
        '   ',
        '/repo/a',
        ' /repo/b ',
      ]),
    ).toEqual(['/repo/a', '/repo/b'])
  })
})

describe('filterTargetsForScope', () => {
  it('keeps global targets unchanged', () => {
    expect(
      filterTargetsForScope(
        { cursor: true, hermes: true, codex: true },
        tools,
        'global',
      ),
    ).toEqual({ cursor: true, hermes: true, codex: true })
  })

  it('unselects tools that do not support project scope', () => {
    expect(
      filterTargetsForScope(
        { cursor: true, hermes: true, codex: true },
        tools,
        'project',
      ),
    ).toEqual({ cursor: true, hermes: false, codex: true })
  })
})

describe('buildInstallSyncJobs', () => {
  it('creates one global job per tool', () => {
    expect(buildInstallSyncJobs(['cursor', 'hermes'], 'global', [])).toEqual([
      { toolId: 'cursor', scope: 'global' },
      { toolId: 'hermes', scope: 'global' },
    ])
  })

  it('creates one project job per tool and unique project', () => {
    expect(
      buildInstallSyncJobs(
        ['cursor', 'codex'],
        'project',
        ['/repo/a', '/repo/a', ' /repo/b '],
      ),
    ).toEqual([
      { toolId: 'cursor', scope: 'project', projectPath: '/repo/a' },
      { toolId: 'cursor', scope: 'project', projectPath: '/repo/b' },
      { toolId: 'codex', scope: 'project', projectPath: '/repo/a' },
      { toolId: 'codex', scope: 'project', projectPath: '/repo/b' },
    ])
  })

  it('returns no project jobs when no projects are selected', () => {
    expect(buildInstallSyncJobs(['cursor'], 'project', ['', '   '])).toEqual([])
  })
})
