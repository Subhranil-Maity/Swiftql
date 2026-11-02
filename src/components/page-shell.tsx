import type { ReactNode } from "react";
import { ChevronLeftIcon } from "lucide-react";

import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";

/** Content column shared by the header, body and footer so they line up. */
const COLUMN = "mx-auto w-full max-w-[640px] px-6";

/**
 * Full-height page: a header (back button, title, actions), one scrolling
 * body and an optional footer pinned to the bottom. Every flex child has
 * `min-h-0`, so on a small window the body shrinks and scrolls instead of
 * pushing the footer out of reach.
 */
export function PageShell({
  title,
  subtitle,
  onBack,
  backLabel = "Back",
  actions,
  footer,
  children,
  className,
}: {
  title: ReactNode;
  /** Muted text after the title, e.g. the chosen driver. */
  subtitle?: ReactNode;
  /** Shows a back button when set. */
  onBack?: () => void;
  backLabel?: string;
  /** Right side of the header. */
  actions?: ReactNode;
  footer?: ReactNode;
  children: ReactNode;
  /** Extra classes for the body column. */
  className?: string;
}) {
  return (
    <main className="flex h-svh min-h-0 flex-col bg-background text-foreground">
      <header className="shrink-0">
        <div className={cn(COLUMN, "flex min-h-14 items-center gap-2 pt-6 pb-3")}>
          {onBack && (
            <Button
              variant="ghost"
              size="icon"
              className="-ml-2.5 shrink-0"
              onClick={onBack}
              aria-label={backLabel}
            >
              <ChevronLeftIcon />
            </Button>
          )}
          <div className="flex min-w-0 flex-1 items-baseline gap-2">
            <h1 className="truncate text-xl font-semibold tracking-tight">{title}</h1>
            {subtitle && (
              <span className="truncate text-muted-foreground">{subtitle}</span>
            )}
          </div>
          {actions && <div className="flex shrink-0 items-center gap-1">{actions}</div>}
        </div>
      </header>

      <ScrollArea className="min-h-0 flex-1">
        <div className={cn(COLUMN, "flex flex-col gap-6 pt-3 pb-8", className)}>{children}</div>
      </ScrollArea>

      {footer && (
        <footer className="shrink-0 border-t bg-background">
          <div className={cn(COLUMN, "py-3")}>{footer}</div>
        </footer>
      )}
    </main>
  );
}
