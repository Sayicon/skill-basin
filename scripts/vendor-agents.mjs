#!/usr/bin/env node

/**
 * Vendors vercel-labs/skills' src/agents.ts (MIT) into basin/agents.json shape.
 *
 * agents.ts is real TypeScript (computed globalSkillsDir via join(homeVar, '...'),
 * async detectInstalled functions) — not a plain data literal — so this does a
 * lightweight structural parse (brace-depth entry splitter + per-field regex)
 * instead of pulling in a TS toolchain. Every entry keeps `verified: false` and
 * a `source` URL pointing at the exact vendored commit; nothing here is trusted
 * blindly (see docs/superpowers/specs/2026-07-08-faz3-agent-adapter-db-design.md).
 *
 * Tier1 (claude_code/cursor/antigravity/hermes_agent) is Kerem-verified by hand
 * in Rust and is ALWAYS excluded from the output — see DECISIONS.md D11.
 */

export const TIER1_KEYS = ['claude_code', 'cursor', 'antigravity', 'hermes_agent']

const AGENTS_OBJECT_MARKER = 'export const agents'

/** kebab-case vercel keys -> snake_case, to line up with ToolId::as_key(). */
export function normalizeKey(key) {
  return key.replace(/-/g, '_')
}

/**
 * Finds `const xHome = <env override> || join(home, '.literal')` declarations
 * and returns { xHome: '.literal', ... }. `home` itself always maps to ''.
 */
export function parseHomeVarBases(source) {
  const bases = { home: '' }
  const re = /const (\w+Home)\s*=\s*[^\n;]*?join\(home,\s*'([^']+)'\)\s*;/g
  let match
  while ((match = re.exec(source))) {
    bases[match[1]] = match[2]
  }
  return bases
}

/**
 * Splits `export const agents: Record<...> = { key: {...}, key2: {...} }`
 * into [key, bodyText] pairs using brace/paren depth counting (no string-aware
 * escaping needed — every literal in this file is a plain path/label string).
 */
export function splitTopLevelEntries(source) {
  const markerIdx = source.indexOf(AGENTS_OBJECT_MARKER)
  if (markerIdx === -1) {
    throw new Error('agents object literal not found (export const agents ...)')
  }
  const braceStart = source.indexOf('{', markerIdx)
  if (braceStart === -1) {
    throw new Error('opening brace for agents object not found')
  }

  const entries = []
  let i = braceStart + 1

  while (i < source.length) {
    while (i < source.length && /[\s,]/.test(source[i])) i++
    if (source[i] === '}') break // end of outer object

    let key
    if (source[i] === "'" || source[i] === '"') {
      const quote = source[i]
      let j = i + 1
      while (source[j] !== quote) j++
      key = source.slice(i + 1, j)
      i = j + 1
    } else {
      let j = i
      while (/[\w$]/.test(source[j])) j++
      key = source.slice(i, j)
      i = j
    }

    while (/\s/.test(source[i])) i++
    if (source[i] !== ':') {
      throw new Error(`expected ':' after key "${key}" near index ${i}`)
    }
    i++
    while (/\s/.test(source[i])) i++
    if (source[i] !== '{') {
      throw new Error(`expected '{' for entry "${key}" near index ${i}`)
    }

    const bodyStart = i + 1
    let depth = 1
    let j = bodyStart
    while (depth > 0 && j < source.length) {
      const c = source[j]
      if (c === '{' || c === '(') depth++
      else if (c === '}' || c === ')') depth--
      j++
    }
    entries.push([key, source.slice(bodyStart, j - 1)])
    i = j
  }

  return entries
}

function extractStringField(body, fieldName) {
  const match = body.match(new RegExp(`${fieldName}:\\s*'([^']*)'`))
  return match ? match[1] : null
}

/**
 * Resolves the entry's `globalSkillsDir` expression against known home-var
 * bases. Returns one of:
 *   { kind: 'resolved', path: '~/<base>/<suffix>' }
 *   { kind: 'none' }         — literal `undefined` (project-scope only tool)
 *   { kind: 'unsupported' }  — a function call or unknown expression
 */
export function extractGlobalSkillsDir(body, homeBases) {
  if (/globalSkillsDir:\s*undefined/.test(body)) {
    return { kind: 'none' }
  }
  const match = body.match(/globalSkillsDir:\s*join\(\s*(\w+)\s*,\s*'([^']*)'\s*\)/)
  if (!match) {
    return { kind: 'unsupported' }
  }
  const [, homeVar, suffix] = match
  if (!(homeVar in homeBases)) {
    return { kind: 'unsupported' }
  }
  const base = homeBases[homeVar]
  const path = base ? `~/${base}/${suffix}` : `~/${suffix}`
  return { kind: 'resolved', path, detectBase: base || suffix.split('/')[0] }
}

/**
 * Pure transform: agents.ts source text -> { registry, skipped }.
 * `registry` matches basin/agents.json shape; `skipped` lists keys that
 * couldn't be confidently vendored (needs a manual agents.json entry).
 */
export function transformAgentsTs(source, { sourceUrl, tier1Keys = TIER1_KEYS } = {}) {
  const homeBases = parseHomeVarBases(source)
  const entries = splitTopLevelEntries(source)
  const adapters = {}
  const skipped = []

  for (const [rawKey, body] of entries) {
    const key = normalizeKey(rawKey)
    if (tier1Keys.includes(key)) continue

    const displayName = extractStringField(body, 'displayName')
    const skillsDir = extractStringField(body, 'skillsDir')
    if (!displayName || !skillsDir) {
      skipped.push({ key, reason: 'missing displayName or skillsDir' })
      continue
    }

    const global = extractGlobalSkillsDir(body, homeBases)
    if (global.kind === 'unsupported') {
      skipped.push({ key, reason: 'globalSkillsDir is not a simple join(home, literal) expression' })
      continue
    }

    adapters[key] = {
      displayName,
      adapterKind: 'dir',
      ...(global.kind === 'resolved' ? { globalSkillsDir: global.path } : {}),
      projectSkillsDir: skillsDir,
      detect: global.kind === 'resolved' ? [`~/${global.detectBase}`] : [],
      defaultStrategy: 'auto',
      verified: false,
      source: sourceUrl,
      custom: false,
    }
  }

  return { registry: { version: 1, adapters }, skipped }
}
