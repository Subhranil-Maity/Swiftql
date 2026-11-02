import { Empty, EmptyDescription, EmptyHeader } from "@/components/ui/empty";

/** Where a tab's results will show. Placeholder until queries can run. */
export function OutputPanel() {
  return (
    <section aria-label="Output" className="flex h-full min-h-0 flex-col bg-muted/30">
      <header className="flex h-9 shrink-0 items-center px-4 text-muted-foreground">Output</header>
      {/* TODO: results grid (glide-data-grid is already a dependency), server messages and timing */}
      <Empty className="min-h-0 p-4">
        <EmptyHeader>
          <EmptyDescription>Run a query to see its results here.</EmptyDescription>
        </EmptyHeader>
      </Empty>
    </section>
  );
}
