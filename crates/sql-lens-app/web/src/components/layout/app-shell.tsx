import { Outlet } from "react-router-dom";

import { useSidebar } from "@/app/providers/sidebar-provider";
import { DetailDrawer } from "@/components/layout/detail-drawer";
import { Sidebar } from "@/components/layout/sidebar";
import { SidebarNav } from "@/components/layout/sidebar-nav";
import { Topbar } from "@/components/layout/topbar";
import {
  Sheet,
  SheetContent,
  SheetTitle,
} from "@/components/ui/sheet";

export function AppShell() {
  const { isMobileOpen, closeMobile } = useSidebar();

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background text-foreground">
      {/* Desktop sidebar — hidden on < md */}
      <Sidebar />

      {/* Mobile nav Sheet — visible only on < md via the hamburger trigger */}
      <Sheet open={isMobileOpen} onOpenChange={(open) => !open && closeMobile()}>
        <SheetContent side="left" className="w-64 p-0 sm:max-w-64">
          <SheetTitle className="sr-only">Navigation</SheetTitle>
          <div className="flex h-12 items-center border-b px-4 font-semibold tracking-tight">
            SQL Lens
          </div>
          <SidebarNav onNavigate={closeMobile} />
        </SheetContent>
      </Sheet>

      {/* Main area */}
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar />
        <main className="flex-1 overflow-auto p-4 md:p-6">
          <Outlet />
        </main>
      </div>

      {/* Right-side detail drawer */}
      <DetailDrawer />
    </div>
  );
}
