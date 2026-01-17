import { clsx } from 'clsx'

interface NavItemProps {
  icon: React.ComponentType<{ className?: string }>
  label: string
  active: boolean
  onClick: () => void
}

export function NavItem({ icon: Icon, label, active, onClick }: NavItemProps) {
  return (
    <button
      onClick={onClick}
      className={clsx(
        'w-full flex items-center gap-3 px-3 py-2 rounded-lg transition-colors text-left',
        active
          ? 'bg-primary-50 text-primary-700 font-medium'
          : 'text-muted-foreground hover:bg-muted'
      )}
    >
      <Icon className="h-5 w-5" />
      {label}
    </button>
  )
}
