import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty";
import { Spinner } from "@/components/ui/spinner";

/** What the Tables and Graph views show before their first model arrives. */
export function ModelLoadState({ error, onRetry }: { error: string | null; onRetry: () => void }) {
  if (!error) {
    return (
      <div className="flex flex-1 items-center justify-center gap-2 text-muted-foreground">
        <Spinner />
        Reading the schema…
      </div>
    );
  }
  return (
    <Empty>
      <EmptyHeader>
        <EmptyTitle>Couldn't read the schema</EmptyTitle>
        <EmptyDescription className="font-mono text-xs">{error}</EmptyDescription>
        <EmptyDescription>
          If a transaction failed in the console, roll it back and try again.
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <Button variant="outline" size="sm" onClick={onRetry}>
          Try again
        </Button>
      </EmptyContent>
    </Empty>
  );
}
