import type { DigestItem, DigestResponse } from '../api'

export interface PDFDayGroup {
  date: Date
  dateLabel: string
  items: DigestItem[]
}

export interface ExportPDFOptions {
  digest: DigestResponse
  type: 'daily' | 'weekly'
  dateLabel: string
  dayGroups?: PDFDayGroup[]
}
