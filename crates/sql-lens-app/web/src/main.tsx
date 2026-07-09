import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";

import { QueryClientProvider } from "@tanstack/react-query";

import App from "./App";
import { DetailDrawerProvider } from "@/app/providers/detail-drawer-provider";
import { SidebarProvider } from "@/app/providers/sidebar-provider";
import { ThemeProvider } from "@/app/providers/theme-provider";
import { TooltipProvider } from "@/components/ui/tooltip";
import { Toaster } from "@/components/ui/sonner";
import { queryClient } from "@/lib/query-client";
import "@/styles/globals.css";

const rootEl = document.getElementById("root");
if (!rootEl) throw new Error("Root element #root not found");

createRoot(rootEl).render(
  <StrictMode>
    <ThemeProvider>
      <SidebarProvider>
        <DetailDrawerProvider>
          <QueryClientProvider client={queryClient}>
            <TooltipProvider>
              <BrowserRouter>
                <App />
              </BrowserRouter>
              <Toaster richColors closeButton />
            </TooltipProvider>
          </QueryClientProvider>
        </DetailDrawerProvider>
      </SidebarProvider>
    </ThemeProvider>
  </StrictMode>,
);
