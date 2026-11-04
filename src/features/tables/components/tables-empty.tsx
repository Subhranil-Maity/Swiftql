import { Button } from "@/components/ui/button";
import {
  Empty,
  EmptyContent,
  EmptyDescription,
  EmptyHeader,
  EmptyTitle,
} from "@/components/ui/empty";
import { plural } from "@/lib/format-count";

/** Nothing to list: an empty database, or a search without matches. */
export function TablesEmpty({
  query,
  total,
  onClear,
}: {
  query: string;
  /** Tables in the database. */
  total: number;
  onClear: () => void;
}) {
  if (total === 0) {
    return (
      <Empty>
        <EmptyHeader>
          <EmptyTitle>No tables yet</EmptyTitle>
          <EmptyDescription>
            Create one with CREATE TABLE in the console. This list refreshes after it runs.
          </EmptyDescription>
        </EmptyHeader>
      </Empty>
    );
  }
  return (
    <Empty>
      <EmptyHeader>
        <EmptyTitle>No tables match "{query.trim()}"</EmptyTitle>
        <EmptyDescription>
          Clear the search to see all {plural(total, "table")}. Use schema.table to search one schema.
        </EmptyDescription>
      </EmptyHeader>
      <EmptyContent>
        <Button variant="outline" size="sm" onClick={onClear}>
          Clear search
        </Button>
      </EmptyContent>
    </Empty>
  );
}
