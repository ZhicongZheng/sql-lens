import { PanelLeftClose, PanelLeft } from "lucide-react";

import { useSidebar } from "@/app/providers/sidebar-provider";
import { Button } from "@/components/ui/button";
import { SidebarNav } from "@/components/layout/sidebar-nav";

export function Sidebar() {
  const { isCollapsed, toggleCollapse } = useSidebar();

  return (
    <aside
      className={`hidden h-full shrink-0 flex-col border-r bg-card transition-[width] duration-200 md:flex ${
        isCollapsed ? "w-16" : "w-56"
      }`}
    >
      <div className="flex h-12 items-center justify-between border-b px-4">
        {!isCollapsed && (
          <span className="font-semibold tracking-tight">SQL Lens</span>
        )}
        <Button
          variant="ghost"
          size="icon"
          onClick={toggleCollapse}
          aria-label={isCollapsed ? "Expand sidebar" : "Collapse sidebar"}
          className={isCollapsed ? "mx-auto" : "ml-auto"}
        >
          {isCollapsed ? (
            <PanelLeft className="size-4" />
          ) : (
            <PanelLeftClose className="size-4" />
          )}
        </Button>
      </div>
      <SidebarNav collapsed={isCollapsed} />
    </aside>
  );
}
