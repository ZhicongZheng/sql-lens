import { useState } from "react";
import ReactECharts from "echarts-for-react";
import type { EChartsOption } from "echarts";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { ToggleGroup, ToggleGroupItem } from "@/components/ui/toggle-group";

type TimeWindow = "1h" | "6h" | "24h" | "7d";

interface StatisticsData {
  qps: Array<{ timestamp: string; value: number }>;
  latency: Array<{ timestamp: string; p50: number; p95: number; p99: number }>;
  errorRate: Array<{ timestamp: string; value: number }>;
  topFingerprints: Array<{ fingerprint: string; count: number }>;
}

const mockData: StatisticsData = {
  qps: [
    { timestamp: "10:00", value: 45 },
    { timestamp: "10:05", value: 52 },
    { timestamp: "10:10", value: 38 },
    { timestamp: "10:15", value: 61 },
    { timestamp: "10:20", value: 55 },
  ],
  latency: [
    { timestamp: "10:00", p50: 12, p95: 45, p99: 120 },
    { timestamp: "10:05", p50: 15, p95: 52, p99: 135 },
    { timestamp: "10:10", p50: 10, p95: 38, p99: 95 },
    { timestamp: "10:15", p50: 18, p95: 65, p99: 180 },
    { timestamp: "10:20", p50: 14, p95: 48, p99: 125 },
  ],
  errorRate: [
    { timestamp: "10:00", value: 0.5 },
    { timestamp: "10:05", value: 0.8 },
    { timestamp: "10:10", value: 0.3 },
    { timestamp: "10:15", value: 1.2 },
    { timestamp: "10:20", value: 0.6 },
  ],
  topFingerprints: [
    { fingerprint: "SELECT * FROM users WHERE id = ?", count: 1250 },
    { fingerprint: "INSERT INTO orders (user_id, amount) VALUES (?, ?)", count: 890 },
    { fingerprint: "UPDATE products SET stock = stock - ? WHERE id = ?", count: 620 },
    { fingerprint: "SELECT COUNT(*) FROM events WHERE created_at > ?", count: 480 },
  ],
};

function getQpsOption(data: StatisticsData): EChartsOption {
  return {
    tooltip: { trigger: "axis" },
    xAxis: { type: "category", data: data.qps.map((d) => d.timestamp) },
    yAxis: { type: "value", name: "QPS" },
    series: [
      {
        name: "QPS",
        type: "line",
        data: data.qps.map((d) => d.value),
        smooth: true,
        areaStyle: { opacity: 0.2 },
      },
    ],
  };
}

function getLatencyOption(data: StatisticsData): EChartsOption {
  return {
    tooltip: { trigger: "axis" },
    legend: { data: ["p50", "p95", "p99"] },
    xAxis: { type: "category", data: data.latency.map((d) => d.timestamp) },
    yAxis: { type: "value", name: "Latency (ms)" },
    series: [
      { name: "p50", type: "line", data: data.latency.map((d) => d.p50), smooth: true },
      { name: "p95", type: "line", data: data.latency.map((d) => d.p95), smooth: true },
      { name: "p99", type: "line", data: data.latency.map((d) => d.p99), smooth: true },
    ],
  };
}

function getErrorRateOption(data: StatisticsData): EChartsOption {
  return {
    tooltip: { trigger: "axis" },
    xAxis: { type: "category", data: data.errorRate.map((d) => d.timestamp) },
    yAxis: { type: "value", name: "Error Rate (%)" },
    series: [
      {
        name: "Error Rate",
        type: "line",
        data: data.errorRate.map((d) => d.value),
        smooth: true,
        itemStyle: { color: "#ef4444" },
      },
    ],
  };
}

function getTopFingerprintsOption(data: StatisticsData): EChartsOption {
  return {
    tooltip: { trigger: "item" },
    xAxis: { type: "value" },
    yAxis: {
      type: "category",
      data: data.topFingerprints.map((d) => d.fingerprint.substring(0, 40) + "..."),
    },
    series: [
      {
        type: "bar",
        data: data.topFingerprints.map((d) => d.count),
        itemStyle: { color: "#3b82f6" },
      },
    ],
  };
}

export function StatisticsPage() {
  const [window, setWindow] = useState<TimeWindow>("1h");
  const [data] = useState<StatisticsData>(mockData);

  const hasData = data.qps.length > 0;

  if (!hasData) {
    return (
      <div className="p-6">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Statistics</h1>
          <p className="text-sm text-muted-foreground">
            QPS, latency percentiles, error rate, and top fingerprints over time.
          </p>
        </div>
        <div className="mt-8 flex h-64 items-center justify-center rounded-lg border border-dashed">
          <p className="text-sm text-muted-foreground">No statistics data available</p>
        </div>
      </div>
    );
  }

  return (
    <div className="p-6 space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-semibold tracking-tight">Statistics</h1>
          <p className="text-sm text-muted-foreground">
            QPS, latency percentiles, error rate, and top fingerprints over time.
          </p>
        </div>
        <ToggleGroup
          type="single"
          value={window}
          onValueChange={(v) => v && setWindow(v as TimeWindow)}
        >
          <ToggleGroupItem value="1h">1h</ToggleGroupItem>
          <ToggleGroupItem value="6h">6h</ToggleGroupItem>
          <ToggleGroupItem value="24h">24h</ToggleGroupItem>
          <ToggleGroupItem value="7d">7d</ToggleGroupItem>
        </ToggleGroup>
      </div>

      <div className="grid grid-cols-1 gap-6 lg:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle>Query Volume (QPS)</CardTitle>
            <CardDescription>Queries per second over time</CardDescription>
          </CardHeader>
          <CardContent>
            <ReactECharts option={getQpsOption(data)} style={{ height: 300 }} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Latency Percentiles</CardTitle>
            <CardDescription>p50, p95, p99 latency over time</CardDescription>
          </CardHeader>
          <CardContent>
            <ReactECharts option={getLatencyOption(data)} style={{ height: 300 }} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Error Rate</CardTitle>
            <CardDescription>Percentage of failed queries over time</CardDescription>
          </CardHeader>
          <CardContent>
            <ReactECharts option={getErrorRateOption(data)} style={{ height: 300 }} />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Top Fingerprints</CardTitle>
            <CardDescription>Most frequent query patterns</CardDescription>
          </CardHeader>
          <CardContent>
            <ReactECharts option={getTopFingerprintsOption(data)} style={{ height: 300 }} />
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
