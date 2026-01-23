import { Menu, MenuButton, MenuItem, MenuItems } from '@headlessui/react'
import { Download, FileText, FileDown, ChevronDown } from 'lucide-react'
import { Button } from './ui/Button'

export interface ExportMenuProps {
  onExportPDF: () => void
  onExportMarkdown: () => void
  disabled?: boolean
  isExporting?: boolean
}

export function ExportMenu({
  onExportPDF,
  onExportMarkdown,
  disabled = false,
  isExporting = false,
}: ExportMenuProps) {
  return (
    <Menu as="div" className="relative">
      {({ open }) => (
        <>
          <MenuButton
            as={Button}
            variant="outline"
            size="sm"
            disabled={disabled || isExporting}
            aria-label="Export options"
          >
            <Download className="h-4 w-4 mr-1.5" />
            {isExporting ? 'Exporting...' : 'Export'}
            <ChevronDown
              className={`h-3.5 w-3.5 ml-1 transition-transform ${open ? 'rotate-180' : ''}`}
            />
          </MenuButton>

      <MenuItems
        anchor="bottom end"
        className="z-50 mt-1 w-44 origin-top-right rounded-lg border border-border bg-card shadow-lg focus:outline-none"
      >
        <div className="p-1">
          <MenuItem>
            {({ focus }) => (
              <button
                onClick={onExportMarkdown}
                className={`${
                  focus ? 'bg-muted' : ''
                } group flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-foreground`}
              >
                <FileText className="h-4 w-4 text-muted-foreground" />
                Markdown (.md)
              </button>
            )}
          </MenuItem>
          <MenuItem>
            {({ focus }) => (
              <button
                onClick={onExportPDF}
                className={`${
                  focus ? 'bg-muted' : ''
                } group flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-foreground`}
              >
                <FileDown className="h-4 w-4 text-muted-foreground" />
                PDF (.pdf)
              </button>
            )}
          </MenuItem>
        </div>
      </MenuItems>
        </>
      )}
    </Menu>
  )
}
