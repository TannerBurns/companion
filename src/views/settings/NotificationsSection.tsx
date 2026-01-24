import { Bell, Clock } from 'lucide-react'

export function NotificationsSection() {
  return (
    <div>
      <div className="mb-6">
        <h3 className="text-lg font-semibold text-foreground">Notifications</h3>
        <p className="text-sm text-muted-foreground mt-1">
          Configure when and how you receive notifications.
        </p>
      </div>

      <div className="space-y-4">
        <div className="flex items-center justify-between p-4 bg-card border border-border rounded-lg opacity-75">
          <div className="flex items-center gap-3">
            <Bell className="h-5 w-5 text-muted-foreground" />
            <div>
              <div className="flex items-center gap-2">
                <h4 className="font-medium text-foreground">
                  Daily Digest Notification
                </h4>
                <span className="inline-flex items-center gap-1 text-xs text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-900/20 px-2 py-0.5 rounded-full">
                  <Clock className="h-3 w-3" />
                  Coming Soon
                </span>
              </div>
              <p className="text-sm text-muted-foreground">
                Get notified when your daily digest is ready
              </p>
            </div>
          </div>
          <button
            disabled
            className="relative inline-flex h-6 w-11 items-center rounded-full transition-colors bg-gray-300 dark:bg-gray-600 cursor-not-allowed"
          >
            <span className="inline-block h-4 w-4 transform rounded-full bg-white translate-x-1" />
          </button>
        </div>
      </div>
    </div>
  )
}
