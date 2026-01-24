import '@testing-library/jest-dom/vitest'
import { vi } from 'vitest'

vi.mock('@tauri-apps/plugin-shell', () => ({
  open: vi.fn().mockResolvedValue(undefined),
}))
