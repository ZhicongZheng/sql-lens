import { Button } from "@/components/ui/button";
import { useTheme } from "@/app/providers/theme-provider";

export function Topbar() {
  const { theme, toggleTheme } = useTheme();

  return (
    <header className="flex h-12 items-center justify-between border-b bg-card px-4">
      <div className="flex items-center gap-3 text-sm text-muted-foreground">
        {/* Placeholder for active target + capture status + global search (UI.md). */}
        <span className="font-medium text-foreground">Capture</span>
        <span className="text-status-unknown">idle</span>
      </div>
      <Button
        variant="outline"
        size="sm"
        onClick={toggleTheme}
        aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
      >
        {theme === "dark" ? "Light" : "Dark"}
      </Button>
    </header>
  );
}
