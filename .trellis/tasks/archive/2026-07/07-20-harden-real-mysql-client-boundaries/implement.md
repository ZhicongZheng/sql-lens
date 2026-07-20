# Implement

1. Add `docker_mysql_session_identity_and_prepare_reuse_are_captured` in `mysql_live_docker.rs`.
2. Flow: create table → prep SELECT or UPDATE → exec twice → close → assert events + connection identity via `/api/v1/connections`.
3. Validate unit tests without Docker; note docker optional in summary.
