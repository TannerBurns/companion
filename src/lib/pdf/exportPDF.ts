import { pdf } from '@react-pdf/renderer'
import { DigestPDF } from './DigestPDF'
import type { ExportPDFOptions } from './types'

export async function exportDigestPDF({
  digest,
  type,
  dateLabel,
  dayGroups,
}: ExportPDFOptions): Promise<void> {
  const doc = DigestPDF({ digest, type, dateLabel, dayGroups })
  const blob = await pdf(doc).toBlob()

  const sanitizedDate = dateLabel.replace(/[^a-zA-Z0-9-]/g, '_')
  const filename = `${type === 'daily' ? 'daily-digest' : 'weekly-summary'}_${sanitizedDate}.pdf`

  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  document.body.appendChild(link)
  link.click()
  document.body.removeChild(link)
  URL.revokeObjectURL(url)
}
