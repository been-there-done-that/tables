// Engine types matching the backend
export type DatabaseEngine = "postgres" | "sqlite" | "mongodb" | "redis" | "elasticsearch" | "s3";

// Base connection draft for frontend forms
export interface ConnectionDraft {
  name: string;
  engine: DatabaseEngine;
  config: unknown; // Engine-specific config that will be validated
}

// PostgreSQL configuration
export interface PostgresConfig {
  version: 1;
  db: {
    host: string;
    port?: number;
    database: string;
    username: string;
  };
  transport: PostgresTransport;
  tls?: PostgresTls;
  options?: Record<string, unknown>;
}

export interface PostgresTransport {
  type: "direct" | "ssh";
  ssh?: {
    host: string;
    port?: number;
    user: string;
    auth: PostgresSshAuth;
  };
}

export interface PostgresSshAuth {
  type: "key" | "password" | "agent";
  key_ref?: string;
  password_ref?: string;
}

export interface PostgresTls {
  enabled: boolean;
  sslmode?: "disable" | "allow" | "prefer" | "require" | "verify-ca" | "verify-full";
  ca_ref?: string;
  cert_ref?: string;
  key_ref?: string;
}

// SQLite configuration
export interface SqliteConfig {
  version: 1;
  mode: "file" | "memory";
  file?: string;
  options?: {
    read_only?: boolean;
    pragmas?: Record<string, unknown>;
  };
}

// Runtime connection types (for type safety)
export type RuntimeConnection = 
  | { engine: "postgres"; config: PostgresConfig }
  | { engine: "sqlite"; config: SqliteConfig };

// Connection summary for list views
export interface ConnectionSummary {
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  uses_ssh: boolean;
  uses_tls: boolean;
}

// Full connection model (as stored in database)
export interface Connection {
  id: string;
  name: string;
  engine: string;
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  uses_ssh: boolean;
  uses_tls: boolean;
  config_json: string;
  is_favorite: boolean;
  color_tag?: string;
  created_at: number;
  updated_at: number;
  last_connected_at?: number;
  connection_count: number;
}

// Connection form state
export interface ConnectionFormState {
  draft: ConnectionDraft;
  errors: Record<string, string>;
  is_valid: boolean;
}

// Engine capabilities for UI generation
export interface EngineCapabilities {
  id: DatabaseEngine;
  name: string;
  supports_ssh: boolean;
  supports_tls: boolean;
  supports_database: boolean;
  supports_username: boolean;
  supports_port: boolean;
  default_port?: number;
  config_schema: ConfigSchema;
}

export interface ConfigSchema {
  sections: ConfigSection[];
}

export interface ConfigSection {
  id: string;
  title: string;
  description?: string;
  fields: ConfigField[];
  condition?: (config: unknown) => boolean; // Show/hide condition
}

export interface ConfigField {
  id: string;
  label: string;
  type: "text" | "number" | "password" | "select" | "checkbox" | "textarea" | "secret_ref";
  required?: boolean;
  default?: unknown;
  options?: Array<{ value: string; label: string }>;
  placeholder?: string;
  validation?: {
    min?: number;
    max?: number;
    pattern?: string;
    message?: string;
  };
  secret_type?: "password" | "ssh_key" | "ssl_cert" | "api_token"; // For secret_ref fields
  condition?: (config: unknown) => boolean; // Show/hide condition
}

// Engine capabilities definitions
export const ENGINE_CAPABILITIES: Record<DatabaseEngine, EngineCapabilities> = {
  postgres: {
    id: "postgres",
    name: "PostgreSQL",
    supports_ssh: true,
    supports_tls: true,
    supports_database: true,
    supports_username: true,
    supports_port: true,
    default_port: 5432,
    config_schema: {
      sections: [
        {
          id: "database",
          title: "Database Connection",
          fields: [
            {
              id: "db.host",
              label: "Host",
              type: "text",
              required: true,
              placeholder: "localhost"
            },
            {
              id: "db.port",
              label: "Port",
              type: "number",
              default: 5432,
              validation: { min: 1, max: 65535 }
            },
            {
              id: "db.database",
              label: "Database",
              type: "text",
              required: true,
              placeholder: "my_database"
            },
            {
              id: "db.username",
              label: "Username",
              type: "text",
              required: true,
              placeholder: "postgres"
            }
          ]
        },
        {
          id: "transport",
          title: "Transport",
          fields: [
            {
              id: "transport.type",
              label: "Connection Type",
              type: "select",
              default: "direct",
              options: [
                { value: "direct", label: "Direct Connection" },
                { value: "ssh", label: "SSH Tunnel" }
              ]
            }
          ]
        },
        {
          id: "ssh",
          title: "SSH Tunnel",
          condition: (config: any) => config?.transport?.type === "ssh",
          fields: [
            {
              id: "transport.ssh.host",
              label: "SSH Host",
              type: "text",
              required: true,
              placeholder: "bastion.example.com"
            },
            {
              id: "transport.ssh.port",
              label: "SSH Port",
              type: "number",
              default: 22,
              validation: { min: 1, max: 65535 }
            },
            {
              id: "transport.ssh.user",
              label: "SSH User",
              type: "text",
              required: true,
              placeholder: "ubuntu"
            },
            {
              id: "transport.ssh.auth.type",
              label: "Authentication",
              type: "select",
              default: "key",
              options: [
                { value: "key", label: "SSH Key" },
                { value: "password", label: "Password" },
                { value: "agent", label: "SSH Agent" }
              ]
            },
            {
              id: "transport.ssh.auth.key_ref",
              label: "SSH Key Reference",
              type: "secret_ref",
              secret_type: "ssh_key",
              condition: (config: any) => config?.transport?.ssh?.auth?.type === "key"
            },
            {
              id: "transport.ssh.auth.password_ref",
              label: "Password Reference",
              type: "secret_ref",
              secret_type: "password",
              condition: (config: any) => config?.transport?.ssh?.auth?.type === "password"
            }
          ]
        },
        {
          id: "tls",
          title: "TLS/SSL",
          fields: [
            {
              id: "tls.enabled",
              label: "Enable TLS",
              type: "checkbox",
              default: false
            },
            {
              id: "tls.sslmode",
              label: "SSL Mode",
              type: "select",
              condition: (config: any) => config?.tls?.enabled,
              options: [
                { value: "disable", label: "Disable" },
                { value: "allow", label: "Allow" },
                { value: "prefer", label: "Prefer" },
                { value: "require", label: "Require" },
                { value: "verify-ca", label: "Verify CA" },
                { value: "verify-full", label: "Verify Full" }
              ]
            },
            {
              id: "tls.ca_ref",
              label: "CA Certificate Reference",
              type: "secret_ref",
              secret_type: "ssl_cert",
              condition: (config: any) => config?.tls?.enabled
            }
          ]
        },
        {
          id: "options",
          title: "Advanced Options",
          fields: [
            {
              id: "options.search_path",
              label: "Search Path",
              type: "text",
              placeholder: "public"
            },
            {
              id: "options.application_name",
              label: "Application Name",
              type: "text",
              placeholder: "MyApp"
            }
          ]
        }
      ]
    }
  },
  sqlite: {
    id: "sqlite",
    name: "SQLite",
    supports_ssh: false,
    supports_tls: false,
    supports_database: false, // Uses file path instead
    supports_username: false,
    supports_port: false,
    config_schema: {
      sections: [
        {
          id: "mode",
          title: "Database Mode",
          fields: [
            {
              id: "mode",
              label: "Mode",
              type: "select",
              required: true,
              default: "file",
              options: [
                { value: "file", label: "File Database" },
                { value: "memory", label: "In-Memory Database" }
              ]
            }
          ]
        },
        {
          id: "file",
          title: "File Settings",
          condition: (config: any) => config?.mode === "file",
          fields: [
            {
              id: "file",
              label: "Database File",
              type: "text",
              required: true,
              placeholder: "/path/to/database.db"
            }
          ]
        },
        {
          id: "options",
          title: "Options",
          fields: [
            {
              id: "options.read_only",
              label: "Read Only",
              type: "checkbox",
              default: false
            },
            {
              id: "options.pragmas.journal_mode",
              label: "Journal Mode",
              type: "select",
              options: [
                { value: "DELETE", label: "DELETE" },
                { value: "WAL", label: "WAL" },
                { value: "MEMORY", label: "MEMORY" }
              ]
            },
            {
              id: "options.pragmas.foreign_keys",
              label: "Foreign Keys",
              type: "checkbox",
              default: true
            }
          ]
        }
      ]
    }
  },
  mongodb: {
    id: "mongodb",
    name: "MongoDB",
    supports_ssh: true,
    supports_tls: true,
    supports_database: true,
    supports_username: true,
    supports_port: true,
    default_port: 27017,
    config_schema: {
      sections: [] // TODO: Implement MongoDB schema
    }
  },
  redis: {
    id: "redis",
    name: "Redis",
    supports_ssh: true,
    supports_tls: true,
    supports_database: false, // Uses database number
    supports_username: false,
    supports_port: true,
    default_port: 6379,
    config_schema: {
      sections: [] // TODO: Implement Redis schema
    }
  },
  elasticsearch: {
    id: "elasticsearch",
    name: "Elasticsearch",
    supports_ssh: true,
    supports_tls: true,
    supports_database: false,
    supports_username: true,
    supports_port: true,
    default_port: 9200,
    config_schema: {
      sections: [] // TODO: Implement Elasticsearch schema
    }
  },
  s3: {
    id: "s3",
    name: "Amazon S3",
    supports_ssh: false,
    supports_tls: true, // Always enabled
    supports_database: false, // Uses bucket instead
    supports_username: false,
    supports_port: false,
    config_schema: {
      sections: [] // TODO: Implement S3 schema
    }
  }
};

// Helper functions
export function getEngineCapabilities(engine: DatabaseEngine): EngineCapabilities {
  return ENGINE_CAPABILITIES[engine];
}

export function createEmptyDraft(engine: DatabaseEngine): ConnectionDraft {
  const capabilities = getEngineCapabilities(engine);
  
  // Start with basic structure
  const draft: ConnectionDraft = {
    name: "",
    engine,
    config: {
      version: 1
    } as any
  };

  // Apply defaults from schema
  for (const section of capabilities.config_schema.sections) {
    for (const field of section.fields) {
      if (field.default !== undefined) {
        setNestedValue(draft.config, field.id, field.default);
      }
    }
  }

  return draft;
}

export function setNestedValue(obj: any, path: string, value: any) {
  const keys = path.split('.');
  let current = obj;
  
  for (let i = 0; i < keys.length - 1; i++) {
    const key = keys[i];
    if (!(key in current)) {
      current[key] = {};
    }
    current = current[key];
  }
  
  current[keys[keys.length - 1]] = value;
}

export function getNestedValue(obj: any, path: string): any {
  const keys = path.split('.');
  let current = obj;
  
  for (const key of keys) {
    if (current && typeof current === 'object' && key in current) {
      current = current[key];
    } else {
      return undefined;
    }
  }
  
  return current;
}
