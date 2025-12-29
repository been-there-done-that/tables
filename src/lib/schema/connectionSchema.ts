// Database engine types
export type DatabaseEngine = "postgres" | "mysql" | "sqlite" | "mongodb" | "redis" | "elasticsearch" | "s3";

// PostgreSQL config structure
export interface PostgresConfig {
  version: 1;
  db: {
    host: string;
    port?: number;
    database: string;
    username: string;
    password?: string;
  };
  transport: {
    type: "direct" | "ssh";
    ssh?: {
      host: string;
      port?: number;
      user: string;
      auth: {
        type: "key" | "password" | "agent";
        key_ref?: string;
        password_ref?: string;
      };
    };
  };
  tls?: {
    enabled: boolean;
    sslmode?: "disable" | "allow" | "prefer" | "require" | "verify-ca" | "verify-full";
    ca_ref?: string;
    cert_ref?: string;
    key_ref?: string;
  };
  options?: {
    search_path?: string;
    application_name?: string;
  };
}

// MySQL config structure
export interface MysqlConfig {
  version: 1;
  db: {
    host: string;
    port?: number;
    database: string;
    username: string;
    password?: string;
  };
  transport: {
    type: "direct" | "ssh";
    ssh?: {
      host: string;
      port?: number;
      user: string;
      auth: {
        type: "key" | "password" | "agent";
        key_ref?: string;
        password_ref?: string;
      };
    };
  };
  tls?: {
    enabled: boolean;
    sslmode?: "DISABLED" | "PREFERRED" | "REQUIRED" | "VERIFY_CA" | "VERIFY_IDENTITY";
    ca_ref?: string;
  };
}

// SQLite config structure
export interface SqliteConfig {
  version: 1;
  mode: "file" | "memory";
  file?: string;
  options?: {
    read_only?: boolean;
    pragmas?: {
      journal_mode?: string;
      foreign_keys?: boolean;
    };
  };
}

// Form field definitions for each engine
export const ENGINE_SCHEMAS = {
  postgres: {
    name: "PostgreSQL",
    fields: {
      "db.host": { label: "Host", type: "text", required: true, default: "localhost" },
      "db.port": { label: "Port", type: "number", default: 5432, min: 1, max: 65535 },
      "db.database": { label: "Database", type: "text", required: true },
      "db.username": { label: "Username", type: "text", required: true },
      "db.password": { label: "Password", type: "secret" },
      "transport.type": { label: "Connection Type", type: "select", default: "direct", options: ["direct", "ssh"] },
      "transport.ssh.host": { label: "SSH Host", type: "text", required: true, condition: (config: any) => config?.transport?.type === "ssh" },
      "transport.ssh.port": { label: "SSH Port", type: "number", default: 22, condition: (config: any) => config?.transport?.type === "ssh" },
      "transport.ssh.user": { label: "SSH User", type: "text", required: true, condition: (config: any) => config?.transport?.type === "ssh" },
      "transport.ssh.auth.type": { label: "SSH Auth", type: "select", default: "key", options: ["key", "password", "agent"], condition: (config: any) => config?.transport?.type === "ssh" },
      "transport.ssh.auth.key_ref": { label: "SSH Key Ref", type: "secret", condition: (config: any) => config?.transport?.type === "ssh" && config?.transport?.ssh?.auth?.type === "key" },
      "transport.ssh.auth.password_ref": { label: "Password Ref", type: "secret", condition: (config: any) => config?.transport?.type === "ssh" && config?.transport?.ssh?.auth?.type === "password" },
      "tls.enabled": { label: "Enable TLS", type: "checkbox", default: false },
      "tls.sslmode": { label: "SSL Mode", type: "select", options: ["disable", "allow", "prefer", "require", "verify-ca", "verify-full"], condition: (config: any) => config?.tls?.enabled },
      "tls.ca_ref": { label: "CA Certificate", type: "secret", condition: (config: any) => config?.tls?.enabled },
      "options.search_path": { label: "Search Path", type: "text", default: "public" },
      "options.application_name": { label: "Application Name", type: "text" }
    }
  },

  mysql: {
    name: "MySQL",
    fields: {
      "db.host": { label: "Host", type: "text", required: true, default: "localhost" },
      "db.port": { label: "Port", type: "number", default: 3306, min: 1, max: 65535 },
      "db.database": { label: "Database", type: "text", required: true },
      "db.username": { label: "Username", type: "text", required: true },
      "db.password": { label: "Password", type: "secret" },
      "transport.type": { label: "Connection Type", type: "select", default: "direct", options: ["direct", "ssh"] },
      "transport.ssh.host": { label: "SSH Host", type: "text", required: true, condition: (config: any) => config?.transport?.type === "ssh" },
      "transport.ssh.port": { label: "SSH Port", type: "number", default: 22, condition: (config: any) => config?.transport?.type === "ssh" },
      "transport.ssh.user": { label: "SSH User", type: "text", required: true, condition: (config: any) => config?.transport?.type === "ssh" },
      "transport.ssh.auth.type": { label: "SSH Auth", type: "select", default: "key", options: ["key", "password", "agent"], condition: (config: any) => config?.transport?.type === "ssh" },
      "transport.ssh.auth.key_ref": { label: "SSH Key Ref", type: "secret", condition: (config: any) => config?.transport?.type === "ssh" && config?.transport?.ssh?.auth?.type === "key" },
      "transport.ssh.auth.password_ref": { label: "Password Ref", type: "secret", condition: (config: any) => config?.transport?.type === "ssh" && config?.transport?.ssh?.auth?.type === "password" },
      "tls.enabled": { label: "Enable TLS", type: "checkbox", default: false },
      "tls.sslmode": { label: "SSL Mode", type: "select", options: ["DISABLED", "PREFERRED", "REQUIRED", "VERIFY_CA", "VERIFY_IDENTITY"], condition: (config: any) => config?.tls?.enabled },
      "tls.ca_ref": { label: "CA Certificate", type: "secret", condition: (config: any) => config?.tls?.enabled }
    }
  },

  sqlite: {
    name: "SQLite",
    fields: {
      "mode": { label: "Mode", type: "select", required: true, default: "file", options: ["file", "memory"] },
      "file": { label: "Database File", type: "text", required: true, condition: (config: any) => config?.mode === "file" },
      "options.read_only": { label: "Read Only", type: "checkbox", default: false },
      "options.pragmas.journal_mode": { label: "Journal Mode", type: "select", options: ["DELETE", "WAL", "MEMORY"] },
      "options.pragmas.foreign_keys": { label: "Foreign Keys", type: "checkbox", default: true }
    }
  },

  mongodb: {
    name: "MongoDB",
    fields: {
      "auth.method": { label: "Connect Method", type: "select", default: "standard", options: ["standard", "uri"] },
      "db.uri": { label: "Connection URI", type: "text", required: true, placeholder: "mongodb+srv://...", condition: (config: any) => config?.auth?.method === "uri" },
      "db.host": { label: "Host", type: "text", required: true, default: "localhost", condition: (config: any) => config?.auth?.method === "standard" },
      "db.port": { label: "Port", type: "number", default: 27017, condition: (config: any) => config?.auth?.method === "standard" },
      "db.srv": { label: "Use SRV", type: "checkbox", default: false, condition: (config: any) => config?.auth?.method === "standard" },
      "db.database": { label: "Database", type: "text", required: true },
      "db.username": { label: "Username", type: "text" },
      "db.password": { label: "Password", type: "secret" },
      "db.authSource": { label: "Auth Database", type: "text", placeholder: "admin" },
      "transport.type": { label: "Connection Type", type: "select", default: "direct", options: ["direct", "ssh"] },
      "tls.enabled": { label: "Enable TLS", type: "checkbox", default: false }
    }
  },

  redis: {
    name: "Redis",
    fields: {
      "db.host": { label: "Host", type: "text", required: true, default: "localhost" },
      "db.port": { label: "Port", type: "number", default: 6379, min: 1, max: 65535 },
      "db.database": { label: "Database Index", type: "number", default: 0 },
      "db.username": { label: "Username (ACL)", type: "text" },
      "db.password": { label: "Password", type: "secret" },
      "transport.type": { label: "Connection Type", type: "select", default: "direct", options: ["direct", "ssh"] },
      "tls.enabled": { label: "Enable TLS", type: "checkbox", default: false }
    }
  },

  elasticsearch: {
    name: "Elasticsearch",
    fields: {
      "auth.method": { label: "Auth Method", type: "select", default: "basic", options: ["basic", "api_key", "cloud_id"] },
      "db.cloud_id": { label: "Cloud ID", type: "text", required: true, condition: (config: any) => config?.auth?.method === "cloud_id" },
      "db.host": { label: "Host", type: "text", required: true, default: "localhost", condition: (config: any) => config?.auth?.method !== "cloud_id" },
      "db.port": { label: "Port", type: "number", default: 9200, condition: (config: any) => config?.auth?.method !== "cloud_id" },
      "db.username": { label: "Username", type: "text", condition: (config: any) => config?.auth?.method === "basic" },
      "db.password": { label: "Password", type: "secret", condition: (config: any) => config?.auth?.method === "basic" },
      "db.api_key": { label: "API Key", type: "secret", required: true, condition: (config: any) => config?.auth?.method === "api_key" },
      "transport.type": { label: "Connection Type", type: "select", default: "direct", options: ["direct", "ssh"] },
      "tls.enabled": { label: "Enable TLS", type: "checkbox", default: false },
      "tls.ca_fingerprint": { label: "CA Fingerprint", type: "text", condition: (config: any) => config?.tls?.enabled }
    }
  },

  s3: {
    name: "Amazon S3",
    fields: {
      "endpoint": { label: "Endpoint", type: "text", required: true },
      "bucket": { label: "Bucket", type: "text", required: true },
      "region": { label: "Region", type: "text", default: "us-east-1" },
      "access_key_id": { label: "Access Key ID", type: "secret" },
      "secret_access_key": { label: "Secret Access Key", type: "secret" }
    }
  }
} as const;

// Helper functions
export function createEmptyConfig(engine: DatabaseEngine) {
  if (engine === "postgres") {
    return {
      version: 1,
      db: { host: "localhost", port: 5432, database: "", username: "", password: "" },
      transport: { type: "direct" as const },
      tls: { enabled: false },
      options: { search_path: "public", application_name: "" }
    };
  }

  if (engine === "mysql") {
    return {
      version: 1,
      db: { host: "localhost", port: 3306, database: "", username: "", password: "" },
      transport: { type: "direct" as const },
      tls: { enabled: false }
    };
  }

  if (engine === "sqlite") {
    return {
      version: 1,
      mode: "file" as const,
      file: "",
      options: { read_only: false, pragmas: { journal_mode: "WAL", foreign_keys: true } }
    };
  }

  if (engine === "mongodb") {
    return {
      version: 1,
      auth: { method: "standard" as const },
      db: { host: "localhost", port: 27017, database: "", username: "", password: "", authSource: "admin", srv: false },
      transport: { type: "direct" as const },
      tls: { enabled: false }
    };
  }

  if (engine === "redis") {
    return {
      version: 1,
      db: { host: "localhost", port: 6379, database: 0, username: "", password: "" },
      transport: { type: "direct" as const },
      tls: { enabled: false }
    };
  }

  if (engine === "elasticsearch") {
    return {
      version: 1,
      auth: { method: "basic" as const },
      db: { host: "localhost", port: 9200, username: "", password: "", api_key: "", cloud_id: "" },
      transport: { type: "direct" as const },
      tls: { enabled: false, ca_fingerprint: "" }
    };
  }

  if (engine === "s3") {
    return {
      version: 1,
      endpoint: "",
      bucket: "",
      region: "us-east-1",
      access_key_id: "",
      secret_access_key: ""
    };
  }

  return { version: 1 };
}

export function getFieldsForEngine(engine: DatabaseEngine) {
  return ENGINE_SCHEMAS[engine]?.fields || {};
}

export function isFieldVisible(engine: DatabaseEngine, fieldPath: string, config: any): boolean {
  const schema = ENGINE_SCHEMAS[engine];
  if (!schema) return false;

  const field = schema.fields[fieldPath as keyof typeof schema.fields];
  if (!field) return false;

  const fieldDef = field as any;
  if (fieldDef.condition) {
    return fieldDef.condition(config);
  }

  return true;
}
