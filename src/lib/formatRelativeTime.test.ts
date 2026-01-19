import { describe, it, expect } from 'vitest'
import { formatRelativeTime } from './formatRelativeTime'

describe('formatRelativeTime', () => {
  const baseTime = new Date('2024-01-15T12:00:00.000Z')
  
  describe('past times (isFuture = false)', () => {
    it('returns "Never" for undefined timestamp', () => {
      expect(formatRelativeTime(undefined, false, baseTime)).toBe('Never')
    })
    
    it('returns "Just now" for less than 1 minute ago', () => {
      const thirtySecondsAgo = baseTime.getTime() - 30 * 1000
      expect(formatRelativeTime(thirtySecondsAgo, false, baseTime)).toBe('Just now')
    })
    
    it('returns "1 minute ago" for exactly 1 minute ago', () => {
      const oneMinuteAgo = baseTime.getTime() - 60 * 1000
      expect(formatRelativeTime(oneMinuteAgo, false, baseTime)).toBe('1 minute ago')
    })
    
    it('returns "5 minutes ago" for 5 minutes ago', () => {
      const fiveMinutesAgo = baseTime.getTime() - 5 * 60 * 1000
      expect(formatRelativeTime(fiveMinutesAgo, false, baseTime)).toBe('5 minutes ago')
    })
    
    it('returns "59 minutes ago" for 59 minutes ago', () => {
      const fiftyNineMinutesAgo = baseTime.getTime() - 59 * 60 * 1000
      expect(formatRelativeTime(fiftyNineMinutesAgo, false, baseTime)).toBe('59 minutes ago')
    })
    
    it('returns "1 hour ago" for exactly 1 hour ago', () => {
      const oneHourAgo = baseTime.getTime() - 60 * 60 * 1000
      expect(formatRelativeTime(oneHourAgo, false, baseTime)).toBe('1 hour ago')
    })
    
    it('returns "5 hours ago" for 5 hours ago', () => {
      const fiveHoursAgo = baseTime.getTime() - 5 * 60 * 60 * 1000
      expect(formatRelativeTime(fiveHoursAgo, false, baseTime)).toBe('5 hours ago')
    })
    
    it('returns "23 hours ago" for 23 hours ago', () => {
      const twentyThreeHoursAgo = baseTime.getTime() - 23 * 60 * 60 * 1000
      expect(formatRelativeTime(twentyThreeHoursAgo, false, baseTime)).toBe('23 hours ago')
    })
    
    it('returns formatted date for 24+ hours ago', () => {
      const twoDaysAgo = baseTime.getTime() - 48 * 60 * 60 * 1000
      const result = formatRelativeTime(twoDaysAgo, false, baseTime)
      expect(result).toMatch(/Jan 13/)
    })
  })
  
  describe('future times (isFuture = true)', () => {
    it('returns "Not scheduled" for undefined timestamp', () => {
      expect(formatRelativeTime(undefined, true, baseTime)).toBe('Not scheduled')
    })
    
    it('returns "Less than a minute" for less than 1 minute ahead', () => {
      const thirtySecondsAhead = baseTime.getTime() + 30 * 1000
      expect(formatRelativeTime(thirtySecondsAhead, true, baseTime)).toBe('Less than a minute')
    })
    
    it('returns "1 minute" for exactly 1 minute ahead', () => {
      const oneMinuteAhead = baseTime.getTime() + 60 * 1000
      expect(formatRelativeTime(oneMinuteAhead, true, baseTime)).toBe('1 minute')
    })
    
    it('returns "5 minutes" for 5 minutes ahead', () => {
      const fiveMinutesAhead = baseTime.getTime() + 5 * 60 * 1000
      expect(formatRelativeTime(fiveMinutesAhead, true, baseTime)).toBe('5 minutes')
    })
    
    it('returns "1 hour" for exactly 1 hour ahead', () => {
      const oneHourAhead = baseTime.getTime() + 60 * 60 * 1000
      expect(formatRelativeTime(oneHourAhead, true, baseTime)).toBe('1 hour')
    })
    
    it('returns "5 hours" for 5 hours ahead', () => {
      const fiveHoursAhead = baseTime.getTime() + 5 * 60 * 60 * 1000
      expect(formatRelativeTime(fiveHoursAhead, true, baseTime)).toBe('5 hours')
    })
    
    it('returns formatted date for 24+ hours ahead', () => {
      const twoDaysAhead = baseTime.getTime() + 48 * 60 * 60 * 1000
      const result = formatRelativeTime(twoDaysAhead, true, baseTime)
      expect(result).toMatch(/Jan 17/)
    })
  })
  
  describe('edge cases', () => {
    it('handles exactly 0 diff as "Just now" for past', () => {
      expect(formatRelativeTime(baseTime.getTime(), false, baseTime)).toBe('Just now')
    })
    
    it('handles exactly 0 diff as "Less than a minute" for future', () => {
      expect(formatRelativeTime(baseTime.getTime(), true, baseTime)).toBe('Less than a minute')
    })
    
    it('handles boundary at 60 minutes (becomes 1 hour)', () => {
      const sixtyMinutesAgo = baseTime.getTime() - 60 * 60 * 1000
      expect(formatRelativeTime(sixtyMinutesAgo, false, baseTime)).toBe('1 hour ago')
    })
    
    it('handles boundary at 24 hours (becomes date)', () => {
      const twentyFourHoursAgo = baseTime.getTime() - 24 * 60 * 60 * 1000
      const result = formatRelativeTime(twentyFourHoursAgo, false, baseTime)
      expect(result).toMatch(/Jan 14/)
    })
  })
})
