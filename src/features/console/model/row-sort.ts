// How a result grid is sorted. The sort itself runs in the backend over every
// held row (`sortResult`; src-tauri/src/results/sort.rs).
export type SortDirection = "asc" | "desc";
export type SortState = { column: number; direction: SortDirection } | null;
