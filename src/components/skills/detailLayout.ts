/**
 * Column widths for the skill detail view, held in memory for the app's
 * lifetime. They survive navigating in and out of the detail view (this module
 * outlives the component) but deliberately reset to the defaults on restart —
 * the module is re-evaluated on a fresh load and nothing is written to disk.
 */
export type DetailLayout = {
  /** File tree column width, px. */
  treeWidth: number
  /** Versions / pin-matrix column width, px. */
  rightWidth: number
}

export const DETAIL_LAYOUT_DEFAULTS: DetailLayout = {
  treeWidth: 236,
  rightWidth: 348,
}

export const DETAIL_LAYOUT_LIMITS = {
  treeWidth: { min: 160, max: 440 },
  rightWidth: { min: 280, max: 620 },
}

/** The live, app-lifetime store. Mutated in place on drag. */
export const detailLayout: DetailLayout = { ...DETAIL_LAYOUT_DEFAULTS }

export const clampWidth = (value: number, min: number, max: number) =>
  Math.max(min, Math.min(max, value))
