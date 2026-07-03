# SQL Lens

> See the SQL your application actually executes.

## Vision

SQL Lens 是一个轻量级、透明的 Developer-first SQL Debug Proxy，专注于开发调试、SQL 可观测性、SQL 审计和 SQL 分析。

SQL Lens 不是数据库中间件，也不是数据库治理平台。它的目标是成为数据库协议世界里的 Charles / Fiddler / mitmproxy。

应用无需修改业务代码，只需要把数据库地址改成 SQL Lens：

```text
Application
      │
      ▼
 SQL Lens
      │
      ▼
Database
```

## Protocol Strategy

SQL Lens 不是只支持 MySQL 协议的工具，而是面向多数据库协议设计。

第一版优先支持 MySQL-compatible protocol，因为 MySQL、StarRocks、TiDB、Apache Doris 等数据库可以共享相近的连接、认证、查询和 Prepared Statement 流程。

后续版本需要为其他协议族预留扩展点：

- PostgreSQL protocol
- SQLite tracing / proxy-compatible integration, subject to technical feasibility
- ClickHouse protocol / HTTP SQL interface
- Other database protocols or SQL execution surfaces

协议设计必须保持可插拔：

- 每个协议族独立实现握手、认证、包解析、命令解析、参数解码和错误映射。
- 上层捕获模型使用统一的 SQL event / connection / statement / parameter 数据结构。
- Web UI、REST API、WebSocket API、存储层和插件系统不直接依赖 MySQL 专有字段。
- 协议专有信息放入可扩展 metadata 字段。

## Initial Supported Databases

v1.0 优先支持 MySQL-compatible protocol 数据库：

- MySQL
- StarRocks
- TiDB
- Apache Doris
- 其他兼容 MySQL protocol 的数据库

未来支持：

- PostgreSQL
- SQLite（如果协议层或驱动集成方式可行）
- ClickHouse

## Product Goals

- 无需修改业务代码。
- 仅修改数据库地址即可接入。
- 捕获应用真实执行的 SQL。
- 自动展开 Prepared Statement 参数。
- 提供 SQL Timeline、SQL Detail、Replay、Slow SQL、Error SQL 和 Statistics。
- 提供现代化 Web UI。
- 默认低资源占用，适合开发环境、本地调试和预发布环境。
- 为后续多协议支持保留清晰边界。

## Technical Stack

### Backend

- Rust first
- Tokio async runtime
- Protocol adapter abstraction
- WebSocket
- REST API
- Optional Go implementation notes only when a component is clearly better suited to Go

### Frontend

- React
- TypeScript
- TailwindCSS
- shadcn/ui
- TanStack Query
- Monaco Editor
- ECharts

## MVP Scope

### Protocol: MySQL-compatible

v1.0 先实现 MySQL-compatible protocol：

- Handshake
- Authentication
- COM_QUERY
- COM_STMT_PREPARE
- COM_STMT_EXECUTE
- COM_STMT_CLOSE
- COM_PING
- COM_QUIT

暂不在 v1.0 实现：

- PostgreSQL protocol
- SQLite integration
- ClickHouse protocol
- SQL rewrite
- Query routing
- Read/write splitting
- Sharding

### Transparent Proxy

```text
Application
      │
      ▼
 SQL Lens
      │
      ▼
MySQL / StarRocks / TiDB / Doris
```

### SQL Capture Model

所有协议适配器都应该输出统一 SQL 事件模型：

- Connection
- Session
- Query
- Prepared Statement
- Execute
- Result Summary
- Error
- Timing
- Protocol Metadata

### Prepared Statement Expansion

输入：

```sql
SELECT * FROM user WHERE id=? AND name=?;
```

参数：

```text
1
Tom
```

展示：

```sql
SELECT * FROM user WHERE id=1 AND name='Tom';
```

参数类型：

- String
- Number
- Boolean
- NULL
- Date
- Timestamp
- JSON
- Blob（摘要显示）

不同协议的参数编码方式不同，但 UI 和存储层应该读取统一后的参数模型。

## Web UI

### Dashboard

- QPS
- TPS
- Connections
- Slow SQL
- Error SQL
- Latency

### SQL List

- 时间
- 协议
- 数据库类型
- 数据库名
- 用户
- 客户端地址
- 耗时
- 行数
- 状态

### SQL Detail

- 原始 SQL
- 展开后 SQL
- 参数列表
- 协议 metadata
- 返回耗时
- 错误信息

### Connections

- Client IP
- Protocol
- User
- Database
- Connection Time
- Bytes In/Out

### Search

支持按：

- SQL
- 协议
- 数据库
- 用户
- 耗时
- 状态
- IP

## API

REST:

- `GET /api/sqls`
- `GET /api/sql/{id}`
- `GET /api/statistics`
- `GET /api/connections`
- `GET /api/protocols`

WebSocket:

- `/ws/sql`
- `/ws/statistics`

API 返回结构应该以通用 SQL capture model 为核心，协议专有字段放入 `metadata`。

## Storage

默认：

- 内存 Ring Buffer（100000 条）

可选：

- SQLite

未来：

- DuckDB

存储模型需要避免强绑定 MySQL 字段，核心索引围绕：

- timestamp
- protocol
- database_type
- database
- user
- client_addr
- normalized_sql
- fingerprint
- status
- latency

## Performance Goals

- 10000+ QPS for capture path in development/staging scenarios
- Proxy overhead target: <1ms p95 under normal local usage
- Default memory target: <100MB for MVP configuration
- Low-copy packet forwarding where practical

## Roadmap

### v1.0

- MySQL-compatible protocol proxy
- SQL capture
- Prepared Statement 参数展开
- Web UI
- Dashboard
- 慢 SQL
- Error SQL
- Ring Buffer storage

### v1.1

- SQL Replay
- SQL Export
- Explain
- SQL Fingerprint
- 参数脱敏
- SQLite storage

### v1.5

- Plugin system
- Prometheus exporter
- OpenTelemetry exporter
- Webhook
- Multi-protocol adapter interface stabilization

### v2.0

- PostgreSQL protocol support
- ClickHouse support research / implementation
- SQLite integration research / implementation
- Advanced analytics with DuckDB

## Non-goals

- 分库分表
- 读写分离
- 高可用
- SQL Rewrite
- 数据同步
- 生产数据库网关替代品
- 数据库权限治理平台

## Recommended Project Structure

```text
sql-lens/
  crates/
    sql-lens-core/
    sql-lens-proxy/
    sql-lens-protocol/
    sql-lens-protocol-mysql/
    sql-lens-storage/
    sql-lens-api/
    sql-lens-plugin/
    sql-lens-app/
  web/
  docs/
  examples/
  tests/
```
