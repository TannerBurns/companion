import { open } from '@tauri-apps/plugin-shell'

export async function openUrl(url: string): Promise<void> {
  try {
    await open(url)
  } catch {
    // Tauri shell not available (e.g., in browser), fall back to window.open
    window.open(url, '_blank', 'noopener,noreferrer')
  }
}
