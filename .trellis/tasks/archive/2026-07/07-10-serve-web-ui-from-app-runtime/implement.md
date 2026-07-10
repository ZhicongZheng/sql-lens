# Implementation Plan

1. Extend `HttpServerConfig` with an optional static directory and preserve it
   when converting `WebConfig`.
2. Add the `tower-http` filesystem feature and configure validated static and
   SPA fallback services in the API router, keeping API/WebSocket fallbacks
   explicit.
3. Add API unit tests for static root/assets/SPA fallback, API fallback
   preservation, and invalid directory errors.
4. Change frontend default URL resolution to browser origin and configure the
   Vite development proxy for API and WebSocket routes; add focused frontend
   tests.
5. Document the built-frontend configuration and one-process startup flow.
6. Validate with Rust format/check/test/clippy and frontend typecheck/test/build.

## Rollback

Setting `web.static_dir` to absent restores existing API-only behavior. The
change neither modifies capture forwarding nor storage persistence.
