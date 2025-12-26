# Database Interface Plan (Rust backend + Svelte frontend)

## Goals
- Datagrip-like desktop DB IDE built with Tauri (Rust backend) and SvelteKit frontend.
- Support many engines via pluggable drivers; UI adapts to engine capabilities.
- Secure, typed, cancellable command/event channel between backend and frontend.

## Backend design (Rust)
- **Engine abstraction**: `DatabaseEngine` trait grouped by capability areas.
  - Connection/session: connect, close, list databases/schemas, set current db/schema.
  - Metadata: list tables/views/materialized views, columns, indexes, constraints/relations, sizes/stats.
  - Query: execute DDL/DML, query with pagination, prepared params, explain/plan, cancel.
  - Monitoring: live stats (connections, locks, slow queries, engine metrics).
  - Admin (optional): create/drop db, users/roles, grants.
- **EngineCapabilities**: flags/optional functions so UI knows what’s supported (schemas? explain? partitions? mat views? cancel?);
  include TypeMapping for display/parameter hints.
- **Engine registry**: map `engine_id` -> implementation (postgres, mysql, sqlite, mssql, oracle, mongo, etc.) with metadata: name, version, auth methods, defaults.
- **Connection/session model**: `ConnectionHandle` keyed by UUID, stored in manager with lifecycle (idle timeout, cleanup). Holds engine id, DSN/config, friendly name, session state.
- **Command surface** (all return envelopes with engine_id, capability_used, duration_ms, warnings):
  - list_engines
  - connect / test_connection / disconnect
  - list_databases, list_schemas, list_tables(filter), get_table_details(columns, indexes, fks, sizes)
  - get_relations_graph (FK graph)
  - get_table_stats (row count, size, bloat if available)
  - run_query (small) + run_query_stream (chunked rows)
  - explain_query
  - cancel_query(request_id)
  - get_server_stats (connections, locks, slow queries, CPU/mem if exposed)
  - set_search_path / use_database
  - export_result (CSV/JSON/Parquet) via streaming
- **Streaming & cancel**: chunked events for large results/long ops; every request has request_id; cancel via command.
- **Error model**: `{ code, message, engine_code?, engine_message?, hint? }` distinguishing user vs system errors.
- **Security**: no secret logging; encrypt stored connection configs (OS keychain preferred); validate inputs (size/time limits); strict Tauri allowlist.

## Frontend integration (Svelte)
- **Transport**: Tauri commands for request/response; events for streaming (`query-chunk`, `query-complete`, `query-error`) and backend push (stats). All tagged with request_id + conn_id.
- **Client abstraction**: `dbClient` module wrapping invoke/listen with `call()` and `stream()`; maintains request registry for cancel and UI state.
- **State**: connections store (capabilities per conn); schema tree store (lazy load); query tabs store (status, rows, columns, errors, timings); metrics store fed by events.
- **Capability-driven UI**: disable/grey features when missing; engine-specific type hints/param editors from TypeMapping.
- **UX/Security**: client-side limits on fetch size; confirm dangerous DDL; redact secrets in toasts/logs.

## Extensibility path
1) Implement `DatabaseEngine` for new DB.
2) Provide `EngineCapabilities` + `TypeMapping`.
3) Register in registry.
4) Frontend auto-adapts via capabilities; only add UI tweaks for engine-unique features.

## Next steps (no implementation yet)
1) Finalize `DatabaseEngine` trait and `EngineCapabilities` schema.
2) Define command/event payload contracts (Rust + TS types) with request_id/cancel protocol.
3) Design connection/session store (timeouts, persistence, keychain usage).
4) Pick first engines (e.g., Postgres, MySQL, SQLite) and map capability gaps.
5) Stub frontend `dbClient` API so UI can start against mocked backend.
