import { NavLink, useNavigate } from "react-router-dom";
import {
  BarChart3,
  Database,
  LayoutDashboard,
  Link,
  Play,
  Settings,
  type LucideIcon,
} from "lucide-react";

import { cn } from "@/lib/utils";
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";

interface NavItem {
  to: string;
  label: string;
  icon: LucideIcon;
}

const NAV_ITEMS: NavItem[] = [
  { to: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
  { to: "/sql", label: "SQL", icon: Database },
  { to: "/connections", label: "Connections", icon: Link },
  { to: "/statistics", label: "Statistics", icon: BarChart3 },
  { to: "/replay", label: "Replay", icon: Play },
  { to: "/settings", label: "Settings", icon: Settings },
];

interface SidebarNavProps {
  /** When true, render icon-only with tooltips. */
  collapsed?: boolean;
  /** Called after navigation (e.g. to close mobile Sheet). */
  onNavigate?: () => void;
}

export function SidebarNav({ collapsed = false, onNavigate }: SidebarNavProps) {
  const navigate = useNavigate();

  return (
    <nav className="flex-1 space-y-1 p-2" aria-label="Primary">
      {NAV_ITEMS.map((item) => {
        const link = (
          <NavLink
            key={item.to}
            to={item.to}
            onClick={() => {
              navigate(item.to);
              onNavigate?.();
            }}
            className={({ isActive }) =>
              cn(
                "flex items-center rounded-md px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-accent text-accent-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground",
                collapsed && "justify-center px-0",
              )
            }
          >
            <item.icon className="size-4 shrink-0" />
            {!collapsed && <span className="ml-3 truncate">{item.label}</span>}
          </NavLink>
        );

        if (collapsed) {
          return (
            <Tooltip key={item.to}>
              <TooltipTrigger asChild>{link}</TooltipTrigger>
              <TooltipContent side="right">{item.label}</TooltipContent>
            </Tooltip>
          );
        }

        return <div key={item.to}>{link}</div>;
      })}
    </nav>
  );
}
