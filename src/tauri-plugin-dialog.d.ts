// The plugin ships its own types, but they resolve to a namespace shape that
// TypeScript will not destructure from a dynamic import here. Declare the
// surface this app uses.
declare module '@tauri-apps/plugin-dialog' {
  type DialogFilter = {
    name: string
    extensions: string[]
  }

  type OpenDialogOptions = {
    directory?: boolean
    multiple?: boolean
    title?: string
    filters?: DialogFilter[]
  }

  type SaveDialogOptions = {
    title?: string
    defaultPath?: string
    filters?: DialogFilter[]
  }

  export function open(options?: OpenDialogOptions): Promise<string | string[] | null>
  /** Resolves to `null` when the user cancels the dialog. */
  export function save(options?: SaveDialogOptions): Promise<string | null>
}
