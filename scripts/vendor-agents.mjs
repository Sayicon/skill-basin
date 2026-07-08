#!/usr/bin/env node

/**
 * Vendors vercel-labs/skills' src/agents.ts (MIT) into basin/agents.json shape.
 *
 * Run: `node scripts/vendor-agents.mjs` — re-fetches the current agents.ts,
 * re-parses it, and overwrites `agents.default.json` at the repo root (same
 * bundling pattern as `featured-skills.json` — committed, embedded via
 * `include_str!`). Not wired into CI; rerun manually when vercel's list
 * changes (GITHUB_TOKEN env var optional, raises the rate limit).
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

// ─── CLI (fetch + write) ───

const REPO = 'vercel-labs/skills'
const FILE_PATH = 'src/agents.ts'
const OUTPUT_FILE = new URL('../agents.default.json', import.meta.url)
const GITHUB_TOKEN = process.env.GITHUB_TOKEN || ''

function githubHeaders(extra = {}) {
  const headers = { 'User-Agent': 'skillbasin-vendor-agents', ...extra }
  if (GITHUB_TOKEN) headers.Authorization = `Bearer ${GITHUB_TOKEN}`
  return headers
}

async function fetchLatestCommitSha() {
  const url = `https://api.github.com/repos/${REPO}/commits?path=${FILE_PATH}&per_page=1`
  const res = await fetch(url, { headers: githubHeaders({ Accept: 'application/vnd.github+json' }) })
  if (!res.ok) {
    throw new Error(`GitHub commits API failed: ${res.status} ${res.statusText}`)
  }
  const commits = await res.json()
  if (!commits[0]?.sha) {
    throw new Error('no commits found for src/agents.ts')
  }
  return commits[0].sha
}

async function fetchAgentsTsAt(sha) {
  const url = `https://raw.githubusercontent.com/${REPO}/${sha}/${FILE_PATH}`
  const res = await fetch(url, { headers: githubHeaders() })
  if (!res.ok) {
    throw new Error(`raw.githubusercontent.com fetch failed: ${res.status} ${res.statusText}`)
  }
  return res.text()
}

async function main() {
  const sha = await fetchLatestCommitSha()
  const source = await fetchAgentsTsAt(sha)
  const sourceUrl = `https://github.com/${REPO}/blob/${sha}/${FILE_PATH}`

  const { registry, skipped } = transformAgentsTs(source, { sourceUrl })

  const { writeFileSync } = await import('node:fs')
  writeFileSync(OUTPUT_FILE, JSON.stringify(registry, null, 2) + '\n')

  const vendoredCount = Object.keys(registry.adapters).length
  console.log(`Vendored ${vendoredCount} agents from ${REPO}@${sha.slice(0, 7)} -> agents.default.json`)
  if (skipped.length) {
    console.warn(`Skipped ${skipped.length} entries (needs a manual agents.json entry if desired):`)
    for (const { key, reason } of skipped) {
      console.warn(`  - ${key}: ${reason}`)
    }
  }
}

const { fileURLToPath } = await import('node:url')
const isMain = process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]
if (isMain) {
  main().catch((err) => {
    console.error(err)
    process.exitCode = 1
  })
}
