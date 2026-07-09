import { Menu, Search, Sun, Moon } from "lucide-react";

import { useSidebar } from "@/app/providers/sidebar-provider";
import { useTheme } from "@/app/providers/theme-provider";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";

export function Topbar() {
  const { theme, toggleTheme } = useTheme();
  const { openMobile } = useSidebar();

  return (
    <header className="flex h-12 shrink-0 items-center gap-2 border-b bg-card px-4">
      {/* Hamburger — mobile only */}
      <Button
        variant="ghost"
        size="icon"
        className="shrink-0 md:hidden"
        onClick={openMobile}
        aria-label="Open navigation"
      >
        <Menu className="size-5" />
      </Button>

      {/* Target badge */}
      <Badge variant="outline" className="shrink-0 gap-1.5 font-mono text-xs">
        <span className="size-1.5 rounded-full bg-primary" />
        mysql-local
      </Badge>

      {/* Capture status */}
      <div className="flex shrink-0 items-center gap-1.5 text-xs">
        <span className="size-1.5 rounded-full bg-status-ok" />
        <span className="text-status-ok">Active</span>
      </div>

      {/* Spacer */}
      <div className="flex-1" />

      {/* Search input — hidden on small screens */}
      <div className="relative hidden sm:block">
        <Search className="pointer-events-none absolute left-2.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" />
        <Input
          placeholder="Search SQL..."
          aria-label="Search SQL"
          className="h-8 w-48 pl-8 text-sm lg:w-64"
        />
      </div>

      {/* Search icon — small screens only */}
      <Button
        variant="ghost"
        size="icon"
        className="shrink-0 sm:hidden"
        aria-label="Search SQL"
      >
        <Search className="size-4" />
      </Button>

      {/* Theme toggle */}
      <Button
        variant="ghost"
        size="icon"
        onClick={toggleTheme}
        aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
        className="shrink-0"
      >
        {theme === "dark" ? (
          <Sun className="size-4" />
        ) : (
          <Moon className="size-4" />
        )}
      </Button>
    </header>
  );
}
