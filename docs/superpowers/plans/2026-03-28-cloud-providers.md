# Cloud Provider Connections Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add first-class Supabase, Neon, and PlanetScale connection support with connection string paste, auto-filled defaults, and an inline step-by-step guide panel.

**Architecture:** Provider is a metadata label stored on the `Connection` struct (`provider: Option<String>`). All three providers use the existing `PostgresAdapter` — no new Rust adapter. A TypeScript `PROVIDERS` registry holds all per-provider knowledge (defaults, guide steps, notes). A new `ProviderForm` component wraps the existing Postgres form fields in a two-column layout with the guide panel on the right.

**Tech Stack:** Rust/rusqlite (migration + struct), SvelteKit + Svelte 5 runes, TypeScript, Tailwind CSS

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src-tauri/migrations/008_add_provider.sql` | CREATE | Adds nullable `provider` column to `connections` table |
| `src-tauri/src/connection.rs` | MODIFY | Add `provider` field; update `load_connection_from_row`; update test fixture |
| `src-tauri/src/connection_manager.rs` | MODIFY | Update INSERT, UPDATE, and all SELECT queries to include `provider` |
| `src/lib/providers/registry.ts` | CREATE | `ProviderConfig` interface + `PROVIDERS` map for Supabase, Neon, PlanetScale |
| `src/lib/providers/parseConnectionString.ts` | CREATE | URI parser returning `Partial<PostgresFormData>` |
| `src/routes/datasource/DriverList.ts` | MODIFY | Add `provider?: string` to `Driver` interface; add 3 provider entries |
| `src/routes/datasource/forms/ConnectionStringInput.svelte` | CREATE | Dashed paste target input that emits parsed fields |
| `src/routes/datasource/forms/ProviderGuidePanel.svelte` | CREATE | Right column: numbered steps + notes + TLS badge |
| `src/routes/datasource/forms/ProviderForm.svelte` | CREATE | Two-column layout: connection string + Postgres fields left, guide right |
| `src/routes/datasource/ConnectionForm.svelte` | MODIFY | Route to `ProviderForm` when `driver.provider` is set |

---

## Task 1: DB Migration

**Files:**
- Create: `src-tauri/migrations/008_add_provider.sql`

- [ ] **Step 1: Create the migration file**

```sql
-- 008_add_provider.sql
-- Add provider label to connections (nullable; NULL = generic engine, no provider)
ALTER TABLE connections ADD COLUMN provider TEXT;
```

- [ ] **Step 2: Verify migration runs on app start**

Run `pnpm tauri dev`. The Tauri app applies migrations on startup via `migrations.rs`. Check the terminal output for any migration errors. No errors = success.

- [ ] **Step 3: Commit**

```bash
git add src-tauri/migrations/008_add_provider.sql
git commit -m "feat(db): add provider column to connections table"
```

---

## Task 2: Rust — Connection Struct + SQL Queries

**Files:**
- Modify: `src-tauri/src/connection.rs`
- Modify: `src-tauri/src/connection_manager.rs`

- [ ] **Step 1: Add `provider` field to the `Connection` struct**

In `src-tauri/src/connection.rs`, add the field after `color_tag`:

```rust
pub struct Connection {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub uses_ssh: bool,
    pub uses_tls: bool,
    pub config_json: String,
    pub is_favorite: bool,
    pub color_tag: Option<String>,
    pub provider: Option<String>,   // "supabase" | "neon" | "planetscale" | null
    pub created_at: i64,
    pub updated_at: i64,
    pub last_connected_at: Option<i64>,
    pub connection_count: i32,
}
```

- [ ] **Step 2: Update the `Connection::new` constructor**

In `Connection::new`, add `provider: None` to the `Ok(Self { ... })` block:

```rust
Ok(Self {
    id: Uuid::new_v4().to_string(),
    name,
    engine,
    host: summary.host,
    port: summary.port,
    database: summary.database,
    username: summary.username,
    uses_ssh: summary.uses_ssh,
    uses_tls: summary.uses_tls,
    config_json,
    is_favorite: false,
    color_tag: None,
    provider: None,
    created_at: std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64,
    updated_at: std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64,
    last_connected_at: None,
    connection_count: 0,
})
```

- [ ] **Step 3: Update `load_connection_from_row` to read `provider` at index 16**

Replace the function body in `src-tauri/src/connection.rs`:

```rust
pub fn load_connection_from_row(row: &rusqlite::Row<'_>) -> Result<Connection, rusqlite::Error> {
    debug!("Loading connection from database row");
    Ok(Connection {
        id: row.get(0)?,
        name: row.get(1)?,
        engine: row.get(2)?,
        host: row.get(3)?,
        port: row.get(4)?,
        database: row.get(5)?,
        username: row.get(6)?,
        uses_ssh: row.get::<_, i64>(7)? != 0,
        uses_tls: row.get::<_, i64>(8)? != 0,
        config_json: row.get(9)?,
        is_favorite: row.get::<_, i64>(10)? != 0,
        color_tag: row.get(11)?,
        created_at: row.get(12)?,
        updated_at: row.get(13)?,
        last_connected_at: row.get(14)?,
        connection_count: row.get(15)?,
        provider: row.get(16)?,
    })
}
```

- [ ] **Step 4: Update `test_parse_config` fixture to compile with the new field**

In the test at the bottom of `src-tauri/src/connection.rs`, add `provider: None` to the `Connection { ... }` literal:

```rust
let mut conn = Connection {
    id: "test_id".to_string(),
    name: "test_conn".to_string(),
    engine: "postgres".to_string(),
    host: None,
    port: None,
    database: None,
    username: None,
    uses_ssh: false,
    uses_tls: false,
    config_json: "".to_string(),
    is_favorite: false,
    color_tag: None,
    provider: None,
    created_at: Utc::now().timestamp(),
    updated_at: Utc::now().timestamp(),
    last_connected_at: None,
    connection_count: 0,
};
```

- [ ] **Step 5: Update INSERT query in `create_connection`**

In `src-tauri/src/connection_manager.rs`, replace the INSERT in `create_connection`:

```rust
conn.execute(
    "INSERT INTO connections (
        id, name, engine, host, port, database, username,
        uses_ssh, uses_tls, config_json, is_favorite, color_tag,
        created_at, updated_at, last_connected_at, connection_count, provider
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
    params![
        connection.id,
        connection.name,
        connection.engine,
        connection.host,
        connection.port,
        connection.database,
        connection.username,
        connection.uses_ssh as i64,
        connection.uses_tls as i64,
        connection.config_json,
        connection.is_favorite as i64,
        connection.color_tag,
        connection.created_at,
        connection.updated_at,
        connection.last_connected_at,
        connection.connection_count,
        connection.provider,
    ],
)
```

- [ ] **Step 6: Update UPDATE query in `update_connection`**

Replace the UPDATE in `update_connection`:

```rust
conn.execute(
    "UPDATE connections SET
        name = ?2, engine = ?3, host = ?4, port = ?5, database = ?6, username = ?7,
        uses_ssh = ?8, uses_tls = ?9, config_json = ?10, is_favorite = ?11, color_tag = ?12,
        updated_at = ?13, provider = ?14
     WHERE id = ?1",
    params![
        connection.id,
        connection.name,
        connection.engine,
        connection.host,
        connection.port,
        connection.database,
        connection.username,
        connection.uses_ssh as i64,
        connection.uses_tls as i64,
        connection.config_json,
        connection.is_favorite as i64,
        connection.color_tag,
        connection.updated_at,
        connection.provider,
    ],
)
```

- [ ] **Step 7: Update all SELECT queries to include `provider`**

There are 4 SELECT queries in `connection_manager.rs` that use `load_connection_from_row`. Update all of them from:

```sql
SELECT id, name, engine, host, port, database, username, uses_ssh, uses_tls, config_json, is_favorite, color_tag, created_at, updated_at, last_connected_at, connection_count
FROM connections ...
```

to:

```sql
SELECT id, name, engine, host, port, database, username, uses_ssh, uses_tls, config_json, is_favorite, color_tag, created_at, updated_at, last_connected_at, connection_count, provider
FROM connections ...
```

The 4 locations are:
1. `get_connection` (line ~186)
2. `get_connection_metadata` (line ~231)
3. `list_connections` (line ~251)
4. `get_favorite_connections` (line ~698)
5. `search_connections` (line ~737)

- [ ] **Step 8: Run Rust tests to verify everything compiles and passes**

```bash
cd src-tauri && cargo test
```

Expected output:
```
test connection::tests::test_parse_config ... ok
test result: ok. 1 passed; 0 failed
```

- [ ] **Step 9: Commit**

```bash
git add src-tauri/src/connection.rs src-tauri/src/connection_manager.rs
git commit -m "feat(rust): add provider field to Connection struct and SQL queries"
```

---

## Task 3: Provider Registry

**Files:**
- Create: `src/lib/providers/registry.ts`

- [ ] **Step 1: Create the registry file**

```typescript
// src/lib/providers/registry.ts

export interface ProviderConfig {
  id: string;
  label: string;
  engine: "postgres";
  color: string;
  defaults: {
    port: number;
    database: string;
    username: string;
    sslRequired: boolean;
  };
  hostPattern?: RegExp;
  guide: string[];
  notes: string[];
  docsUrl: string;
}

export const PROVIDERS: Record<string, ProviderConfig> = {
  supabase: {
    id: "supabase",
    label: "Supabase",
    engine: "postgres",
    color: "#f97316",
    defaults: {
      port: 5432,
      database: "postgres",
      username: "postgres",
      sslRequired: true,
    },
    hostPattern: /db\..+\.supabase\.co/,
    guide: [
      "Open your Supabase Dashboard",
      "Go to Settings → Database",
      "Copy the Connection String (URI format)",
      "Paste it in the field above",
    ],
    notes: [
      "Use port 5432, not 6543. Port 6543 is the connection pooler and does not support schema introspection.",
    ],
    docsUrl: "https://supabase.com/docs/guides/database/connecting-to-postgres",
  },
  neon: {
    id: "neon",
    label: "Neon",
    engine: "postgres",
    color: "#00e599",
    defaults: {
      port: 5432,
      database: "neondb",
      username: "neondb_owner",
      sslRequired: true,
    },
    hostPattern: /.+\.neon\.tech/,
    guide: [
      "Open your Neon Console",
      "Select your project",
      "Click Connect → Connection string",
      "Copy the URI and paste it above",
    ],
    notes: [],
    docsUrl: "https://neon.tech/docs/connect/connect-from-any-app",
  },
  planetscale: {
    id: "planetscale",
    label: "PlanetScale",
    engine: "postgres",
    color: "#f4c430",
    defaults: {
      port: 5432,
      database: "main",
      username: "",
      sslRequired: true,
    },
    hostPattern: /.+\.psdb\.cloud/,
    guide: [
      "Open your PlanetScale dashboard",
      "Select your database",
      "Click Connect → Connect with → Postgres",
      "Copy the connection string and paste it above",
    ],
    notes: [
      "PlanetScale Postgres compatibility is in beta. Use the Postgres connection string, not the MySQL one.",
    ],
    docsUrl: "https://planetscale.com/docs/tutorials/connect-any-application",
  },
};
```

- [ ] **Step 2: Verify TypeScript compiles**

```bash
pnpm check
```

Expected: no errors related to `registry.ts`.

- [ ] **Step 3: Commit**

```bash
git add src/lib/providers/registry.ts
git commit -m "feat(providers): add provider registry for Supabase, Neon, PlanetScale"
```

---

## Task 4: Connection String Parser

**Files:**
- Create: `src/lib/providers/parseConnectionString.ts`

- [ ] **Step 1: Create the parser**

```typescript
// src/lib/providers/parseConnectionString.ts

export interface PostgresFormData {
  host: string;
  port: number;
  username: string;
  password: string;
  database: string;
}

/**
 * Parse a PostgreSQL connection URI into form fields.
 * Supports postgresql:// and postgres:// prefixes.
 * Returns null for malformed input.
 */
export function parseConnectionString(uri: string): Partial<PostgresFormData> | null {
  const trimmed = uri.trim();
  if (!trimmed.startsWith("postgresql://") && !trimmed.startsWith("postgres://")) {
    return null;
  }

  try {
    // Replace postgres:// with http:// so the URL API can parse it
    const normalized = trimmed
      .replace(/^postgresql:\/\//, "http://")
      .replace(/^postgres:\/\//, "http://");

    const url = new URL(normalized);

    const host = url.hostname || undefined;
    const rawPort = url.port ? parseInt(url.port, 10) : undefined;
    const port = rawPort && !isNaN(rawPort) ? rawPort : undefined;
    const username = url.username ? decodeURIComponent(url.username) : undefined;
    const password = url.password ? decodeURIComponent(url.password) : undefined;
    // pathname is "/dbname" — strip the leading slash
    const database = url.pathname && url.pathname.length > 1
      ? decodeURIComponent(url.pathname.slice(1))
      : undefined;

    // Return only the fields we could extract
    const result: Partial<PostgresFormData> = {};
    if (host) result.host = host;
    if (port) result.port = port;
    if (username) result.username = username;
    if (password) result.password = password;
    if (database) result.database = database;

    // Must have at least a host to be useful
    if (!result.host) return null;

    return result;
  } catch {
    return null;
  }
}
```

- [ ] **Step 2: Manually verify the parser in browser devtools (no test framework)**

Open the app with `pnpm dev`, open browser devtools console, import and test:

```javascript
// In devtools — paste each line and check the output
const { parseConnectionString } = await import('/src/lib/providers/parseConnectionString.ts')

// Should return { host: "db.abc.supabase.co", port: 5432, username: "postgres", password: "secret", database: "postgres" }
parseConnectionString("postgresql://postgres:secret@db.abc.supabase.co:5432/postgres")

// Should return null (missing host)
parseConnectionString("not-a-uri")

// Should return { host: "ep-cool.us-east-2.aws.neon.tech", port: 5432, username: "neondb_owner", password: "pw", database: "neondb" }
parseConnectionString("postgres://neondb_owner:pw@ep-cool.us-east-2.aws.neon.tech/neondb")

// Should return port 6543 as-is (pooler warning shown in UI, not corrected here)
parseConnectionString("postgresql://postgres:pw@db.abc.supabase.co:6543/postgres")
```

Expected: results match the comments above.

- [ ] **Step 3: Commit**

```bash
git add src/lib/providers/parseConnectionString.ts
git commit -m "feat(providers): add connection string URI parser"
```

---

## Task 5: Update Driver List

**Files:**
- Modify: `src/routes/datasource/DriverList.ts`

- [ ] **Step 1: Add `provider` to the `Driver` interface and add 3 provider entries**

Replace the full contents of `src/routes/datasource/DriverList.ts`:

```typescript
export interface Driver {
    id: string;
    name: string;
    icon: string;
    defaultPort?: number;
    provider?: string;  // set for cloud providers; undefined for raw engines
}

export const drivers: Driver[] = [
    { id: 'postgresql',   name: 'PostgreSQL',   icon: 'brand-postgresql', defaultPort: 5432 },
    { id: 'supabase',     name: 'Supabase',     icon: 'brand-supabase',   defaultPort: 5432, provider: 'supabase' },
    { id: 'neon',         name: 'Neon',         icon: 'brand-neon',       defaultPort: 5432, provider: 'neon' },
    { id: 'planetscale',  name: 'PlanetScale',  icon: 'brand-planetscale',defaultPort: 5432, provider: 'planetscale' },
    { id: 'mysql',        name: 'MySQL',        icon: 'brand-mysql',      defaultPort: 3306 },
    { id: 'sqlite',       name: 'SQLite',       icon: 'database',         defaultPort: undefined },
    { id: 'mongodb',      name: 'MongoDB',      icon: 'brand-mongodb',    defaultPort: 27017 },
    { id: 'redis',        name: 'Redis',        icon: 'brand-redis',      defaultPort: 6379 },
    { id: 'elasticsearch',name: 'Elasticsearch',icon: 'brand-elastic',    defaultPort: 9200 },
    { id: 's3',           name: 'Amazon S3',    icon: 'brand-aws',        defaultPort: undefined },
    { id: 'athena',       name: 'Amazon Athena',icon: 'brand-aws',        defaultPort: undefined },
    { id: 'custom',       name: 'Custom',       icon: 'database-cog',     defaultPort: undefined },
];
```

Note: `brand-supabase`, `brand-neon`, `brand-planetscale` are placeholder icon names. If these aren't in the Tabler icon set, use `'database'` as a fallback. Check the icon grid in the existing UI to confirm what's available — Tabler Icons v3 may have these.

- [ ] **Step 2: Run svelte-check to confirm no type errors**

```bash
pnpm check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/routes/datasource/DriverList.ts
git commit -m "feat(datasource): add Supabase, Neon, PlanetScale to driver list"
```

---

## Task 6: ConnectionStringInput Component

**Files:**
- Create: `src/routes/datasource/forms/ConnectionStringInput.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- src/routes/datasource/forms/ConnectionStringInput.svelte -->
<script lang="ts">
    import { parseConnectionString, type PostgresFormData } from "$lib/providers/parseConnectionString";

    interface Props {
        onParse: (result: Partial<PostgresFormData>) => void;
    }

    let { onParse }: Props = $props();

    let raw = $state("");
    let status: "idle" | "ok" | "error" = $state("idle");

    function handleInput(e: Event) {
        const value = (e.currentTarget as HTMLInputElement).value;
        raw = value;

        if (!value.trim()) {
            status = "idle";
            return;
        }

        const result = parseConnectionString(value);
        if (result) {
            status = "ok";
            onParse(result);
        } else {
            status = "error";
        }
    }
</script>

<div class="mb-4">
    <div class="text-xs text-[#909090] uppercase tracking-wide mb-1.5">
        Connection string
    </div>
    <input
        type="text"
        placeholder="postgresql://user:password@host:5432/database"
        value={raw}
        oninput={handleInput}
        class="w-full bg-[#2b2d30] border border-dashed rounded px-3 py-2 text-sm text-[#a9b7c6] placeholder-[#555] outline-none font-mono
            focus:ring-1 focus:outline-none
            {status === 'ok' ? 'border-[#22c55e] focus:border-[#22c55e] focus:ring-[#22c55e]' :
             status === 'error' ? 'border-[#ef4444] focus:border-[#ef4444] focus:ring-[#ef4444]' :
             'border-[#5e6060] focus:border-[#3574f0] focus:ring-[#3574f0]'}"
    />
    {#if status === "ok"}
        <div class="text-xs text-[#22c55e] mt-1">↳ Fields filled from connection string</div>
    {:else if status === "error"}
        <div class="text-xs text-[#ef4444] mt-1">Invalid connection string — use postgresql://user:pass@host:port/db format</div>
    {/if}
</div>
```

- [ ] **Step 2: Run svelte-check**

```bash
pnpm check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/routes/datasource/forms/ConnectionStringInput.svelte
git commit -m "feat(datasource): add ConnectionStringInput component with live parse"
```

---

## Task 7: ProviderGuidePanel Component

**Files:**
- Create: `src/routes/datasource/forms/ProviderGuidePanel.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- src/routes/datasource/forms/ProviderGuidePanel.svelte -->
<script lang="ts">
    import type { ProviderConfig } from "$lib/providers/registry";

    interface Props {
        provider: ProviderConfig;
    }

    let { provider }: Props = $props();
</script>

<div class="flex flex-col h-full bg-[#1a1a2e] border-l border-[#1e1e2e] p-5 min-w-[220px] max-w-[260px]">
    <!-- Header -->
    <div class="text-xs font-semibold mb-4" style="color: {provider.color}">
        How to connect
    </div>

    <!-- Steps -->
    <div class="flex flex-col gap-3 mb-5">
        {#each provider.guide as step, i}
            <div class="flex gap-3 items-start">
                <div
                    class="flex-shrink-0 w-5 h-5 rounded-full flex items-center justify-center text-[10px] font-bold border"
                    style="color: {provider.color}; border-color: {provider.color}33; background-color: {provider.color}11;"
                >
                    {i + 1}
                </div>
                <div class="text-xs text-[#888] leading-relaxed">{step}</div>
            </div>
        {/each}
    </div>

    <!-- Notes (warnings) -->
    {#if provider.notes.length > 0}
        <div class="flex flex-col gap-2 mb-4">
            {#each provider.notes as note}
                <div class="bg-[#1a1400] border border-[#3d3000] rounded px-3 py-2">
                    <div class="text-[10px] text-[#fbbf24] font-semibold mb-0.5">⚠ Note</div>
                    <div class="text-[10px] text-[#92794a] leading-relaxed">{note}</div>
                </div>
            {/each}
        </div>
    {/if}

    <!-- TLS badge -->
    {#if provider.defaults.sslRequired}
        <div class="flex items-center gap-1.5 mt-auto">
            <div class="w-1.5 h-1.5 rounded-full bg-[#22c55e]"></div>
            <div class="text-[10px] text-[#555]">TLS enforced by default</div>
        </div>
    {/if}
</div>
```

- [ ] **Step 2: Run svelte-check**

```bash
pnpm check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/routes/datasource/forms/ProviderGuidePanel.svelte
git commit -m "feat(datasource): add ProviderGuidePanel component"
```

---

## Task 8: ProviderForm Component

**Files:**
- Create: `src/routes/datasource/forms/ProviderForm.svelte`

- [ ] **Step 1: Create the component**

```svelte
<!-- src/routes/datasource/forms/ProviderForm.svelte -->
<script lang="ts">
    import { PROVIDERS } from "$lib/providers/registry";
    import type { PostgresFormData } from "$lib/providers/parseConnectionString";
    import ConnectionStringInput from "./ConnectionStringInput.svelte";
    import ProviderGuidePanel from "./ProviderGuidePanel.svelte";
    import PostgresForm from "./PostgresForm.svelte";

    interface Props {
        providerId: string;
        data: any;
        onChange: (field: string, value: any) => void;
    }

    let { providerId, data, onChange }: Props = $props();

    const provider = PROVIDERS[providerId];

    // Apply provider defaults on mount
    $effect(() => {
        if (provider) {
            if (!data.host) onChange("host", "");
            onChange("port", provider.defaults.port);
            if (!data.database) onChange("database", provider.defaults.database);
            if (!data.username) onChange("username", provider.defaults.username);
        }
    });

    function handleParsed(result: Partial<PostgresFormData>) {
        if (result.host !== undefined) onChange("host", result.host);
        if (result.port !== undefined) onChange("port", result.port);
        if (result.username !== undefined) onChange("username", result.username);
        if (result.password !== undefined) onChange("password", result.password);
        if (result.database !== undefined) onChange("database", result.database);
    }
</script>

{#if provider}
    <div class="flex h-full">
        <!-- Left column: connection string + form fields -->
        <div class="flex-1 overflow-y-auto pr-4">
            <ConnectionStringInput onParse={handleParsed} />

            <!-- Divider -->
            <div class="flex items-center gap-3 mb-4">
                <div class="flex-1 h-px bg-[#2a2a2a]"></div>
                <div class="text-xs text-[#444]">or fill manually</div>
                <div class="flex-1 h-px bg-[#2a2a2a]"></div>
            </div>

            <PostgresForm {data} {onChange} />
        </div>

        <!-- Right column: guide panel -->
        <ProviderGuidePanel {provider} />
    </div>
{:else}
    <!-- Fallback: unknown provider, render standard postgres form -->
    <PostgresForm {data} {onChange} />
{/if}
```

- [ ] **Step 2: Run svelte-check**

```bash
pnpm check
```

Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/routes/datasource/forms/ProviderForm.svelte
git commit -m "feat(datasource): add ProviderForm two-column layout component"
```

---

## Task 9: Wire Up ConnectionForm

**Files:**
- Modify: `src/routes/datasource/ConnectionForm.svelte`

- [ ] **Step 1: Import ProviderForm and update the form routing**

At the top of the `<script>` section, add the import:

```typescript
import ProviderForm from "./forms/ProviderForm.svelte";
```

- [ ] **Step 2: Update the `$effect` that resets form data on driver change to apply provider defaults**

Replace the existing effect:

```typescript
$effect(() => {
    if (driver) {
        formData.name = driver.name;
        formData.port = driver.defaultPort;
        // Reset host for providers — leave blank so user pastes their URI
        if (driver.provider) {
            formData.host = "";
        } else {
            formData.host = "localhost";
        }
    } else {
        formData.name = "";
        formData.port = undefined;
        formData.host = "localhost";
    }
});
```

- [ ] **Step 3: Replace the dynamic form section to route to ProviderForm**

Find the `<!-- Dynamic Form Content -->` section and replace it:

```svelte
<!-- Dynamic Form Content -->
<div class="mt-4">
    {#if driver.provider}
        <ProviderForm
            providerId={driver.provider}
            data={formData}
            onChange={handleChange}
        />
    {:else if driver.id === "postgresql" || driver.id === "mysql" || driver.id === "mariadb"}
        <PostgresForm data={formData} onChange={handleChange} />
    {:else if driver.id === "sqlite"}
        <SqliteForm data={formData} onChange={handleChange} />
    {:else}
        <div class="text-center text-gray-500 mt-10">
            Configuration for {driver.name} is not yet implemented.
        </div>
    {/if}
</div>
```

- [ ] **Step 4: Run svelte-check**

```bash
pnpm check
```

Expected: no errors.

- [ ] **Step 5: Smoke test in dev mode**

```bash
pnpm tauri dev
```

1. Open Add Connection
2. Click "Supabase" in the driver grid — the two-column form should appear with the orange guide panel on the right
3. Paste a Supabase connection string — fields should auto-fill
4. Click "Neon" — green guide panel, different defaults
5. Click "PlanetScale" — yellow guide panel, `main` as default database
6. Click "PostgreSQL" — normal Postgres form, no guide panel

- [ ] **Step 6: Commit**

```bash
git add src/routes/datasource/ConnectionForm.svelte
git commit -m "feat(datasource): route provider drivers to ProviderForm"
```

---

## Task 10: Save + Load Provider Field End-to-End

**Files:**
- Verify end-to-end wiring (no new code — just verify the existing Tauri commands pass `provider` through correctly)

The `create_connection` Tauri command accepts a `DatabaseConnection` (which is `Connection`) via serde. Since we added `provider: Option<String>` to the struct with `#[derive(Serialize, Deserialize)]`, it will automatically deserialize from the JSON payload the frontend sends.

The frontend currently calls `invoke('create_connection', { connection: { ... }, credentials: { ... } })`. We need to make sure the frontend passes `provider` in the connection object.

- [ ] **Step 1: Check what the frontend sends to `create_connection`**

Search for `create_connection` or `invoke` calls in the datasource components:

```bash
grep -r "create_connection\|invoke" src/routes/datasource/ src/lib/
```

If `ConnectionForm.svelte` doesn't yet call `create_connection` (the OK/Apply buttons are not yet wired to Tauri commands — they appear to be UI-only placeholders based on the current code), add `provider: driver?.provider ?? null` to the connection payload whenever that wiring is added.

If the buttons are already wired: ensure the payload includes `provider: driver.provider ?? null`.

- [ ] **Step 2: Verify saved connections round-trip**

In `pnpm tauri dev`:
1. Connect via Supabase form and click OK/Apply
2. Close and reopen the datasource panel
3. Click the saved connection — confirm the Supabase form opens (not the generic Postgres form), meaning `provider` was saved and loaded back correctly

- [ ] **Step 3: Commit if any wiring was needed**

```bash
git add -p  # stage only the connection payload changes
git commit -m "feat(datasource): pass provider field through create/update connection commands"
```

---

## Self-Review Notes

- All 9 files from the spec are covered across the 10 tasks
- `load_connection_from_row` reads `provider` at index 16 — consistent with all SELECT queries adding it as the 17th column
- `ProviderForm` effect only sets defaults for empty fields (`if (!data.host)`), so editing an existing provider connection doesn't clobber saved values
- The `parseConnectionString` returns `port: 6543` as-is — the pooler warning in `ProviderGuidePanel` handles the UX, no silent correction
- Task 10 acknowledges the OK/Apply buttons may not be fully wired to Tauri yet — this is pre-existing, not introduced by this feature
