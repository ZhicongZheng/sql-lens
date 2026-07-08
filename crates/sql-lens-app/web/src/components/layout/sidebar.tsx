import { NavLink } from "react-router-dom";

import { cn } from "@/lib/utils";

interface NavItem {
  to: string;
  label: string;
}

// Primary navigation from UI.md "Information Architecture".
const NAV_ITEMS: NavItem[] = [
  { to: "/dashboard", label: "Dashboard" },
  { to: "/sql", label: "SQL" },
  { to: "/connections", label: "Connections" },
  { to: "/statistics", label: "Statistics" },
  { to: "/replay", label: "Replay" },
  { to: "/settings", label: "Settings" },
];

export function Sidebar() {
  return (
    <aside className="flex h-full w-56 shrink-0 flex-col border-r bg-card">
      <div className="flex h-12 items-center border-b px-4 font-semibold tracking-tight">
        SQL Lens
      </div>
      <nav className="flex-1 space-y-1 p-2" aria-label="Primary">
        {NAV_ITEMS.map((item) => (
          <NavLink
            key={item.to}
            to={item.to}
            className={({ isActive }) =>
              cn(
                "flex items-center rounded-md px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-accent text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
              )
            }
          >
            {item.label}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
}
