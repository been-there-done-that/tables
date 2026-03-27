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

    const rawHost = url.hostname;
    const host = rawHost
      ? (rawHost.startsWith('[') && rawHost.endsWith(']')
          ? rawHost.slice(1, -1)
          : rawHost)
      : undefined;
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
