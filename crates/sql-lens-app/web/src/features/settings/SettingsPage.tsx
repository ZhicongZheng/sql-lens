import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";

interface SettingsSectionProps {
  title: string;
  description?: string;
  restartRequired?: boolean;
  children: React.ReactNode;
}

function SettingsSection({ title, description, restartRequired, children }: SettingsSectionProps) {
  return (
    <Card>
      <CardHeader>
        <div className="flex items-center justify-between">
          <div>
            <CardTitle className="text-lg">{title}</CardTitle>
            {description && (
              <CardDescription className="mt-1">{description}</CardDescription>
            )}
          </div>
          {restartRequired && (
            <Badge variant="outline">Restart required</Badge>
          )}
        </div>
      </CardHeader>
      <CardContent className="space-y-4">{children}</CardContent>
    </Card>
  );
}

interface FieldRowProps {
  label: string;
  value: string;
  description?: string;
}

function FieldRow({ label, value, description }: FieldRowProps) {
  return (
    <div className="grid grid-cols-3 gap-4 py-2">
      <div>
        <div className="text-sm font-medium">{label}</div>
        {description && (
          <div className="text-xs text-muted-foreground">{description}</div>
        )}
      </div>
      <div className="col-span-2">
        <div className="rounded-md border bg-muted/50 px-3 py-2 text-sm font-mono">
          {value || <span className="text-muted-foreground italic">not set</span>}
        </div>
      </div>
    </div>
  );
}

export function SettingsPage() {
  return (
    <div className="p-6 space-y-6">
      <div>
        <h1 className="text-2xl font-semibold tracking-tight">Settings</h1>
        <p className="text-sm text-muted-foreground">
          Proxy, backend, storage, redaction, slow SQL threshold, auth, plugins, and exporters.
        </p>
      </div>

      <div className="space-y-6">
        <SettingsSection
          title="Proxy"
          description="Database proxy listener configuration"
          restartRequired
        >
          <FieldRow label="Listen Address" value="127.0.0.1:3307" description="Proxy bind address" />
          <FieldRow label="Protocol" value="mysql" description="Database protocol" />
          <FieldRow label="Capture Mode" value="observe" description="observe | capture" />
          <FieldRow label="Slow Threshold (ms)" value="500" description="Threshold for slow SQL classification" />
          <FieldRow label="Max Connections" value="512" description="Maximum concurrent connections" />
          <FieldRow label="Connect Timeout (ms)" value="5000" description="Backend connection timeout" />
          <FieldRow label="Idle Timeout (ms)" value="300000" description="Connection idle timeout" />
          <FieldRow label="Shutdown Timeout (ms)" value="10000" description="Graceful shutdown timeout" />
        </SettingsSection>

        <SettingsSection
          title="Backend"
          description="Target database connection settings"
          restartRequired
        >
          <FieldRow label="Address" value="127.0.0.1:3306" description="Backend database address" />
          <FieldRow label="Database Type" value="mysql" description="mysql | starrocks | tidb" />
        </SettingsSection>

        <SettingsSection
          title="Storage"
          description="Event storage configuration"
          restartRequired
        >
          <FieldRow label="Type" value="ring_buffer" description="ring_buffer | sqlite" />
          <FieldRow label="Capacity" value="100000" description="Maximum events in ring buffer" />
          <FieldRow label="Path" value="" description="SQLite database path (empty = disabled)" />
        </SettingsSection>

        <SettingsSection
          title="Redaction"
          description="Sensitive data redaction rules"
          restartRequired
        >
          <FieldRow label="Enabled" value="true" description="Enable parameter and SQL redaction" />
          <FieldRow label="Mask" value="***" description="Replacement string for redacted values" />
          <FieldRow label="Parameter Names" value="password, token, secret" description="Parameter names to redact" />
          <FieldRow label="SQL Patterns" value="" description="Regex patterns for SQL redaction" />
        </SettingsSection>

        <SettingsSection
          title="Slow SQL Threshold"
          description="Global threshold for slow query classification"
          restartRequired
        >
          <FieldRow label="Threshold (ms)" value="500" description="Queries exceeding this duration are marked slow" />
        </SettingsSection>

        <SettingsSection
          title="Plugins"
          description="Plugin system configuration"
          restartRequired
        >
          <FieldRow label="Enabled" value="false" description="Enable plugin loading" />
          <FieldRow label="Directory" value="plugins" description="Plugin directory path" />
        </SettingsSection>

        <SettingsSection
          title="Exporters"
          description="Event export destinations"
          restartRequired
        >
          <div className="text-sm text-muted-foreground">
            Exporter configuration will be displayed here when exporters are configured.
          </div>
        </SettingsSection>
      </div>

      <div className="rounded-lg border border-muted bg-muted/30 p-4">
        <p className="text-xs text-muted-foreground">
          Note: This is a read-only settings view. Changes require editing the configuration file and restarting SQL Lens.
        </p>
      </div>
    </div>
  );
}
