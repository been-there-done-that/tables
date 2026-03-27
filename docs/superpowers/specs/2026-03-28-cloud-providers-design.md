# Cloud Provider Connections — Design Spec

**Date:** 2026-03-28
**Status:** Approved
**Scope:** Supabase, Neon, PlanetScale (Postgres-compatible providers only)

---

## Problem

Users connecting to cloud-hosted Postgres providers (Supabase, Neon, PlanetScale) must currently use the generic PostgreSQL form. They have no guidance on where to find their credentials, no pre-filled defaults for that provider, and no connection string paste shortcut. This creates friction and support surface for what should be a 30-second flow.

---

## Goals

- First-class driver entries for Supabase, Neon, PlanetScale in the driver selection grid
- Connection string paste that auto-fills all fields
- Provider-specific defaults (port, TLS, default DB/user) pre-applied
- Inline step-by-step guide panel per provider, no external links required
- Zero new Rust adapters — all three providers are Postgres under the hood

---

## Non-Goals

- Turso, MySQL providers, MongoDB, Redis — out of scope for this pass
- OAuth / API token auth flows — manual credential entry only
- Fetching connection info from provider APIs — user copy-pastes from their dashboard

---

## Architecture

### Option chosen: Provider as metadata

Providers are a UX concept, not an engine concept. The `engine` field stays `"postgres"`. A new `provider` string label (`"supabase"`, `"neon"`, `"planetscale"`, or `null`) is stored alongside it. All provider connections use the existing `PostgresAdapter` — no new Rust adapter, no new config type.

---

## Backend Changes

### 1. DB migration — `src-tauri/migrations/002_add_provider.sql`

```sql
ALTER TABLE connections ADD COLUMN provider TEXT;
```

Nullable — existing connections have `NULL`, new provider connections store the provider id string.

### 2. Connection struct — `src-tauri/src/connection.rs`

Add one field:

```rust
pub provider: Option<String>,   // "supabase" | "neon" | "planetscale" | null
```

Serializes/deserializes as `provider` in JSON. All existing connection create/update/read commands pass this field through unchanged. No logic in Rust — it is purely a label.

---

## Frontend Changes

### 3. Provider registry — `src/lib/providers/registry.ts`

Single source of truth for all provider knowledge.

```ts
interface ProviderConfig {
  id: string           // "supabase" | "neon" | "planetscale"
  label: string
  engine: "postgres"
  color: string        // accent color used in guide panel header
  defaults: {
    port: number
    database: string
    username: string
    sslRequired: boolean
  }
  hostPattern?: RegExp  // for host field placeholder hint
  guide: string[]       // ordered steps shown in guide panel
  notes?: string[]      // warning callouts (e.g. pooler port warning)
  docsUrl: string
}

export const PROVIDERS: Record<string, ProviderConfig> = {
  supabase: {
    id: "supabase",
    label: "Supabase",
    engine: "postgres",
    color: "#f97316",
    defaults: { port: 5432, database: "postgres", username: "postgres", sslRequired: true },
    hostPattern: /db\..+\.supabase\.co/,
    guide: [
      "Open your Supabase Dashboard",
      "Go to Settings → Database",
      "Copy the Connection String (URI format)",
      "Paste it in the field above"
    ],
    notes: [
      "Use port 5432, not 6543. Port 6543 is the connection pooler and does not support schema introspection."
    ],
    docsUrl: "https://supabase.com/docs/guides/database/connecting-to-postgres"
  },
  neon: {
    id: "neon",
    label: "Neon",
    engine: "postgres",
    color: "#00e599",
    defaults: { port: 5432, database: "neondb", username: "neondb_owner", sslRequired: true },
    hostPattern: /.+\.neon\.tech/,
    guide: [
      "Open your Neon Console",
      "Select your project",
      "Click Connect → Connection string",
      "Copy the URI and paste it above"
    ],
    notes: [],
    docsUrl: "https://neon.tech/docs/connect/connect-from-any-app"
  },
  planetscale: {
    id: "planetscale",
    label: "PlanetScale",
    engine: "postgres",
    color: "#f4c430",
    defaults: { port: 5432, database: "main", username: "", sslRequired: true },
    hostPattern: /.+\.psdb\.cloud/,
    guide: [
      "Open your PlanetScale dashboard",
      "Select your database",
      "Click Connect → Connect with → Postgres",
      "Copy the connection string and paste it above"
    ],
    notes: [
      "PlanetScale's Postgres compatibility is in beta. Use the Postgres connection string, not the MySQL one."
    ],
    docsUrl: "https://planetscale.com/docs/tutorials/connect-any-application"
  }
}
```

Adding a new provider in the future = one new object here, zero other changes.

### 4. Connection string parser — `src/lib/providers/parseConnectionString.ts`

```ts
export function parseConnectionString(uri: string): Partial<PostgresFormData> | null
```

- Handles `postgresql://` and `postgres://` prefixes
- Uses the browser `URL` API — no regex parsing of credentials
- Returns `{ host, port, username, password, database }` or `null` for invalid input
- Port `6543` is returned as-is (the UI shows the pooler warning from the provider's `notes`)
- Password travels through the same secure credential path as manual entry — never stored in plaintext

### 5. Driver list — `src/routes/datasource/DriverList.ts`

Add three new entries to the existing driver array:

```ts
{ id: "supabase",     label: "Supabase",     engine: "postgres", provider: "supabase",     icon: "...", defaultPort: 5432 },
{ id: "neon",         label: "Neon",         engine: "postgres", provider: "neon",         icon: "...", defaultPort: 5432 },
{ id: "planetscale",  label: "PlanetScale",  engine: "postgres", provider: "planetscale",  icon: "...", defaultPort: 5432 },
```

They sit flat in the grid alongside PostgreSQL, MySQL, SQLite — no separate section.

### 6. New components

**`ProviderForm.svelte`** (`src/routes/datasource/forms/`)

Two-column layout wrapper:
- Left column: `ConnectionStringInput` (URI paste field with live parse feedback) + `PostgresForm` fields pre-filled with provider defaults
- Right column: `ProviderGuidePanel`
- On mount: applies `PROVIDERS[provider].defaults` to form state
- On connection string input: calls `parseConnectionString`, merges result into form state reactively

**`ConnectionStringInput.svelte`** (`src/routes/datasource/forms/`)

- Single input field styled as a dashed paste target
- Fires `onParse(result)` callback with parsed fields on every `input` event
- Shows green "↳ Fields filled" feedback on success, red "Invalid connection string" on parse failure

**`ProviderGuidePanel.svelte`** (`src/routes/datasource/forms/`)

- Receives a `ProviderConfig` as prop
- Renders numbered guide steps with provider accent color
- Renders `notes` as amber warning callouts
- TLS badge shown when `defaults.sslRequired = true`

**`ConnectionForm.svelte`** (modified)

One change: when `selectedDriver.provider` is set, render `ProviderForm` instead of `PostgresForm`.

---

## End-to-End Flow

1. User opens Add Connection → flat driver grid → clicks "Supabase"
2. `ConnectionForm` detects `driver.provider = "supabase"` → renders `ProviderForm`
3. `ProviderForm` mounts → applies Supabase defaults (port 5432, db "postgres", TLS on)
4. User pastes connection string → `parseConnectionString` fires → merges host/port/user/pass/db into form state
5. If port 6543 detected → pooler warning appears in guide panel
6. User clicks existing Test Connection button → `test_postgres_raw` Tauri command, zero changes
7. User clicks Save → `Connection` stored with `engine: "postgres"`, `provider: "supabase"`, credentials encrypted as before
8. Connection appears in sidebar identically to any Postgres connection

---

## Files Changed / Created

| File | Change |
|------|--------|
| `src-tauri/migrations/002_add_provider.sql` | NEW — adds `provider` column |
| `src-tauri/src/connection.rs` | ADD `provider: Option<String>` field |
| `src/lib/providers/registry.ts` | NEW — provider configs |
| `src/lib/providers/parseConnectionString.ts` | NEW — URI parser |
| `src/routes/datasource/DriverList.ts` | ADD 3 provider entries |
| `src/routes/datasource/forms/ProviderForm.svelte` | NEW — two-column layout |
| `src/routes/datasource/forms/ConnectionStringInput.svelte` | NEW — URI paste field |
| `src/routes/datasource/forms/ProviderGuidePanel.svelte` | NEW — guide panel |
| `src/routes/datasource/ConnectionForm.svelte` | MODIFY — route to ProviderForm |

---

## What This Deliberately Does Not Include

- No API calls to provider services
- No new Rust adapter, enum variant, or config type
- No separate "cloud providers" UI section — flat grid only
- No OAuth or token-based auth flows
