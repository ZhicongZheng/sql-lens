# Protocol Registry Runtime Design

Create and populate a registry in `sql-lens-app` composition. Pass the selected `Arc<dyn ProtocolAdapter>` into each target's forwarding task. The forwarding loop remains protocol-neutral; only the current MySQL-specific framing helpers may need to be moved behind an adapter-owned observation boundary.

Do not add a dependency from config to protocol. Map configured protocol values to protocol names in app composition and convert registry errors into `MinimalMysqlRuntimeError`.
