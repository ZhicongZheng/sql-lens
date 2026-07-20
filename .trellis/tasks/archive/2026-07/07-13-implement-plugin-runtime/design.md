# Plugin Runtime Design

The existing `sql-lens-plugin` traits remain protocol-neutral. Add a runtime-owned dispatcher in `sql-lens-app` or a small plugin runtime module. It receives already-redacted payloads from the capture/lifecycle fan-out and invokes hooks independently, recording failures without returning them to packet forwarding.

Plugin loading must use an explicit supported artifact format. If dynamic native loading is not already available in the workspace, start with a statically registered test/plugin boundary and document the artifact decision rather than adding unsafe loading implicitly.
