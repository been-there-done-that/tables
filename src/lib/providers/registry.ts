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
    hostPattern: /^db\.[^.]+\.supabase\.co$/,
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
    hostPattern: /^.+\.neon\.tech$/,
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
    hostPattern: /^.+\.psdb\.cloud$/,
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
