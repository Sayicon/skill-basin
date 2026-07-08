import { describe, it, expect } from 'vitest'
import {
  normalizeKey,
  parseHomeVarBases,
  splitTopLevelEntries,
  extractGlobalSkillsDir,
  transformAgentsTs,
  TIER1_KEYS,
} from './vendor-agents.mjs'

// Trimmed excerpt of the real vercel-labs/skills src/agents.ts (fetched
// 2026-07-08), kept verbatim so the parser is tested against real shapes:
// home/configHome/claudeHome vars, an `undefined` global dir, a
// function-call global dir (unsupported), and a hyphenated key.
const FIXTURE_SOURCE = `
import { homedir } from 'os';
import { join } from 'path';
import { existsSync, readFileSync, readdirSync } from 'fs';
import { xdgConfig } from 'xdg-basedir';
import type { AgentConfig, AgentType } from './types.ts';

const home = homedir();
const configHome = xdgConfig ?? join(home, '.config');
const claudeHome = process.env.CLAUDE_CONFIG_DIR?.trim() || join(home, '.claude');

function getOpenClawGlobalSkillsDir() {
  return join(home, '.openclaw/skills');
}

export const agents: Record<AgentType, AgentConfig> = {
  'aider-desk': {
    name: 'aider-desk',
    displayName: 'AiderDesk',
    skillsDir: '.aider-desk/skills',
    globalSkillsDir: join(home, '.aider-desk/skills'),
    detectInstalled: async () => {
      return existsSync(join(home, '.aider-desk'));
    },
  },
  amp: {
    name: 'amp',
    displayName: 'Amp',
    skillsDir: '.agents/skills',
    globalSkillsDir: join(configHome, 'agents/skills'),
    detectInstalled: async () => {
      return existsSync(join(configHome, 'amp'));
    },
  },
  'claude-code': {
    name: 'claude-code',
    displayName: 'Claude Code',
    skillsDir: '.claude/skills',
    globalSkillsDir: join(claudeHome, 'skills'),
    detectInstalled: async () => {
      return existsSync(claudeHome);
    },
  },
  openclaw: {
    name: 'openclaw',
    displayName: 'OpenClaw',
    skillsDir: 'skills',
    globalSkillsDir: getOpenClawGlobalSkillsDir(),
    detectInstalled: async () => {
      return (
        existsSync(join(home, '.openclaw')) ||
        existsSync(join(home, '.clawdbot'))
      );
    },
  },
  eve: {
    name: 'eve',
    displayName: 'Eve',
    skillsDir: 'agent/skills',
    globalSkillsDir: undefined,
    detectInstalled: async () => {
      const cwd = process.cwd();
      return existsSync(join(cwd, 'agent'));
    },
  },
};
`

describe('normalizeKey', () => {
  it('converts kebab-case vercel keys to snake_case', () => {
    expect(normalizeKey('aider-desk')).toBe('aider_desk')
    expect(normalizeKey('claude-code')).toBe('claude_code')
    expect(normalizeKey('amp')).toBe('amp')
  })
})

describe('parseHomeVarBases', () => {
  it('extracts home-relative bases for env-overridable home vars', () => {
    const bases = parseHomeVarBases(FIXTURE_SOURCE)
    expect(bases.home).toBe('')
    expect(bases.configHome).toBe('.config')
    expect(bases.claudeHome).toBe('.claude')
  })
})

describe('splitTopLevelEntries', () => {
  it('splits every top-level agent entry despite nested braces/parens', () => {
    const entries = splitTopLevelEntries(FIXTURE_SOURCE)
    const keys = entries.map(([key]) => key)
    expect(keys).toEqual(['aider-desk', 'amp', 'claude-code', 'openclaw', 'eve'])
  })

  it('captures the openclaw body including its multi-line OR detectInstalled', () => {
    const entries = splitTopLevelEntries(FIXTURE_SOURCE)
    const [, body] = entries.find(([key]) => key === 'openclaw')
    expect(body).toContain('.clawdbot')
  })
})

describe('extractGlobalSkillsDir', () => {
  const bases = parseHomeVarBases(FIXTURE_SOURCE)

  it('resolves join(home, literal) to a ~/ path', () => {
    const [, body] = splitTopLevelEntries(FIXTURE_SOURCE).find(([k]) => k === 'aider-desk')
    const result = extractGlobalSkillsDir(body, bases)
    expect(result).toEqual({ kind: 'resolved', path: '~/.aider-desk/skills', detectBase: '.aider-desk' })
  })

  it('resolves join(configHome, literal) using the parsed base', () => {
    const [, body] = splitTopLevelEntries(FIXTURE_SOURCE).find(([k]) => k === 'amp')
    const result = extractGlobalSkillsDir(body, bases)
    expect(result).toEqual({ kind: 'resolved', path: '~/.config/agents/skills', detectBase: '.config' })
  })

  it('flags a bare `undefined` global dir as none', () => {
    const [, body] = splitTopLevelEntries(FIXTURE_SOURCE).find(([k]) => k === 'eve')
    expect(extractGlobalSkillsDir(body, bases)).toEqual({ kind: 'none' })
  })

  it('flags a function-call global dir as unsupported', () => {
    const [, body] = splitTopLevelEntries(FIXTURE_SOURCE).find(([k]) => k === 'openclaw')
    expect(extractGlobalSkillsDir(body, bases)).toEqual({ kind: 'unsupported' })
  })
})

describe('transformAgentsTs', () => {
  const sourceUrl = 'https://github.com/vercel-labs/skills/blob/abc123/src/agents.ts'

  it('emits resolved dir-kind entries with verified:false and source', () => {
    const { registry } = transformAgentsTs(FIXTURE_SOURCE, { sourceUrl })
    expect(registry.version).toBe(1)
    expect(registry.adapters.aider_desk).toEqual({
      displayName: 'AiderDesk',
      adapterKind: 'dir',
      globalSkillsDir: '~/.aider-desk/skills',
      projectSkillsDir: '.aider-desk/skills',
      detect: ['~/.aider-desk'],
      defaultStrategy: 'auto',
      verified: false,
      source: sourceUrl,
      custom: false,
    })
  })

  it('excludes Tier1 keys entirely (claude-code -> claude_code)', () => {
    const { registry } = transformAgentsTs(FIXTURE_SOURCE, { sourceUrl })
    expect(registry.adapters.claude_code).toBeUndefined()
    for (const tier1Key of TIER1_KEYS) {
      expect(registry.adapters[tier1Key]).toBeUndefined()
    }
  })

  it('keeps entries with no global dir, omitting the field', () => {
    const { registry } = transformAgentsTs(FIXTURE_SOURCE, { sourceUrl })
    expect(registry.adapters.eve).toBeDefined()
    expect(registry.adapters.eve.globalSkillsDir).toBeUndefined()
    expect(registry.adapters.eve.projectSkillsDir).toBe('agent/skills')
  })

  it('reports unsupported entries as skipped instead of emitting bad data', () => {
    const { registry, skipped } = transformAgentsTs(FIXTURE_SOURCE, { sourceUrl })
    expect(registry.adapters.openclaw).toBeUndefined()
    expect(skipped).toEqual([
      { key: 'openclaw', reason: 'globalSkillsDir is not a simple join(home, literal) expression' },
    ])
  })
})
