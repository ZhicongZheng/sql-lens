import { Navigate, Route, Routes } from "react-router-dom";

import { AppShell } from "@/components/layout/app-shell";
import { ConnectionsRoute } from "@/app/routes/connections";
import { DashboardRoute } from "@/app/routes/dashboard";
import { ReplayRoute } from "@/app/routes/replay";
import { SettingsRoute } from "@/app/routes/settings";
import { SqlEventsRoute } from "@/app/routes/sql-events";
import { SqlDetailRoute } from "@/app/routes/sql-detail";
import { StatisticsRoute } from "@/app/routes/statistics";

export default function App() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<Navigate to="/dashboard" replace />} />
        <Route path="dashboard" element={<DashboardRoute />} />
        <Route path="sql" element={<SqlEventsRoute />} />
        <Route path="sql/:id" element={<SqlDetailRoute />} />
        <Route path="connections" element={<ConnectionsRoute />} />
        <Route path="statistics" element={<StatisticsRoute />} />
        <Route path="replay" element={<ReplayRoute />} />
        <Route path="settings" element={<SettingsRoute />} />
        <Route
          path="*"
          element={
            <div className="space-y-2">
              <h1 className="text-2xl font-semibold tracking-tight">
                Not found
              </h1>
              <p className="text-sm text-muted-foreground">
                The requested route does not exist.
              </p>
            </div>
          }
        />
      </Route>
    </Routes>
  );
}
