import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";
import type { ConnectionFilter } from "@/lib/api/hooks/use-connections";

interface ConnectionFiltersProps {
  value: ConnectionFilter;
  onChange: (value: ConnectionFilter) => void;
}

export function ConnectionFilters({ value, onChange }: ConnectionFiltersProps) {
  return (
    <ToggleGroup
      type="single"
      value={value}
      onValueChange={(v) => v && onChange(v as ConnectionFilter)}
    >
      <ToggleGroupItem value="active" aria-label="Show active connections">
        Active
      </ToggleGroupItem>
      <ToggleGroupItem value="closed" aria-label="Show closed connections">
        Closed
      </ToggleGroupItem>
    </ToggleGroup>
  );
}
