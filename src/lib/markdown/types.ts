import type { DigestItem, DigestResponse } from '../api'

export interface DayGroup {
  date: Date
  dateLabel: string
  items: DigestItem[]
}

export interface ExportMarkdownOptions {
  digest: DigestResponse
  type: 'daily' | 'weekly'
  dateLabel: string
  dayGroups?: DayGroup[]
}
