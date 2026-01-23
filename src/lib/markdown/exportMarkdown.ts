import { formatDigestMarkdown } from './formatMarkdown'
import type { ExportMarkdownOptions } from './types'

export async function exportDigestMarkdown({
  digest,
  type,
  dateLabel,
  dayGroups,
}: ExportMarkdownOptions): Promise<void> {
  const markdown = formatDigestMarkdown({ digest, type, dateLabel, dayGroups })
  const blob = new Blob([markdown], { type: 'text/markdown;charset=utf-8' })

  const sanitizedDate = dateLabel.replace(/[^a-zA-Z0-9-]/g, '_')
  const filename = `${type === 'daily' ? 'daily-digest' : 'weekly-summary'}_${sanitizedDate}.md`

  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}
