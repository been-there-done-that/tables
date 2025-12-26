# Secure Credential Storage Design

## Overview
Secure credential storage system that stores non-sensitive connection metadata in SQLite and sensitive data (passwords, keys, tokens) in the OS keyring using the `keyring` crate.

## Data Storage Strategy

### SQLite Database (Non-Sensitive Metadata)
- Connection names, hosts, ports, engines
- Usernames (non-sensitive)
- Connection parameters and settings
- UI preferences and organization

### OS Keyring (Sensitive Credentials)
- Database passwords
- SSH private keys and passphrases
- SSL certificates and private keys
- API tokens and authentication secrets

## Keyring Storage Structure
```
connections:{uuid}:password          - Database passwords
connections:{uuid}:ssh_private_key   - SSH private keys
connections:{uuid}:ssh_passphrase    - SSH key passphrases
connections:{uuid}:ssl_cert          - SSL certificates
connections:{uuid}:ssl_private_key   - SSL private keys
connections:{uuid}:ssl_ca_cert        - SSL CA certificates
connections:{uuid}:api_token          - API tokens
```

## Database Schema

```sql
CREATE TABLE connections (
    id TEXT PRIMARY KEY,                    -- UUID
    name TEXT NOT NULL,                     -- Human-readable name
    engine TEXT NOT NULL,                   -- 'postgresql', 'mysql', 'sqlite', etc.
    host TEXT,                              -- NULL for file-based DBs
    port INTEGER,                           -- NULL for file-based DBs
    database TEXT,                          -- Default database name
    username TEXT,                          -- Username (non-sensitive)
    auth_type TEXT NOT NULL DEFAULT 'password',
    ssl_enabled BOOLEAN DEFAULT FALSE,
    ssh_tunnel_enabled BOOLEAN DEFAULT FALSE,
    ssh_tunnel_host TEXT,
    ssh_tunnel_port INTEGER,
    ssh_tunnel_username TEXT,
    connection_params TEXT,                  -- JSON for engine-specific params
    is_favorite BOOLEAN DEFAULT FALSE,
    color_tag TEXT,                         -- UI organization
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_connected_at INTEGER,
    connection_count INTEGER DEFAULT 0
);

-- Performance indexes
CREATE INDEX idx_connections_engine ON connections(engine);
CREATE INDEX idx_connections_name ON connections(name COLLATE NOCASE);
CREATE INDEX idx_connections_favorite ON connections(is_favorite);
CREATE INDEX idx_connections_last_used ON connections(last_connected_at DESC);
```

## Rust Data Structures

### Connection Model
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,                    // UUID
    pub name: String,
    pub engine: DatabaseEngine,
    pub host: Option<String>,
    pub port: Option<u16>,
    pub database: Option<String>,
    pub username: Option<String>,
    pub auth_type: AuthType,
    pub ssl_enabled: bool,
    pub ssh_tunnel_enabled: bool,
    pub ssh_tunnel_host: Option<String>,
    pub ssh_tunnel_port: Option<u16>,
    pub ssh_tunnel_username: Option<String>,
    pub connection_params: HashMap<String, serde_json::Value>,
    pub is_favorite: bool,
    pub color_tag: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_connected_at: Option<i64>,
    pub connection_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabaseEngine {
    PostgreSQL,
    MySQL,
    SQLite,
    MongoDB,
    Redis,
    Elasticsearch,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthType {
    Password,
    SshKey,
    SslCert,
    ApiToken,
    WindowsAuth,
    Kerberos,
    None, // For SQLite or no auth
}
```

### Secure Credentials
```rust
// Sensitive data - never stored in database
#[derive(Clone)]
pub struct SecureCredentials {
    pub password: Option<SecretString>,
    pub ssh_private_key: Option<SecretString>,
    pub ssh_passphrase: Option<SecretString>,
    pub ssl_certificate: Option<SecretString>,
    pub ssl_private_key: Option<SecretString>,
    pub ssl_ca_certificate: Option<SecretString>,
    pub api_token: Option<SecretString>,
}

#[derive(Clone)]
pub struct SecretString {
    inner: String,
}

impl SecretString {
    pub fn new(s: String) -> Self { Self { inner: s } }
    pub fn expose(&self) -> &str { &self.inner }
    // No Debug/Display implementations to prevent accidental exposure
}
```

## Credential Manager

```rust
use keyring::{Entry, Error};

pub struct CredentialManager {
    service_name: String, // e.g., "tables_db_manager"
}

impl CredentialManager {
    pub fn new() -> Self {
        Self {
            service_name: "tables_db_manager".to_string(),
        }
    }

    pub fn store_password(&self, connection_id: &str, password: &str) -> Result<(), Error> {
        let entry = Entry::new(&self.service_name, &format!("connections:{}:password", connection_id))?;
        entry.set_password(password)
    }

    pub fn get_password(&self, connection_id: &str) -> Result<Option<String>, Error> {
        let entry = Entry::new(&self.service_name, &format!("connections:{}:password", connection_id))?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn delete_password(&self, connection_id: &str) -> Result<(), Error> {
        let entry = Entry::new(&self.service_name, &format!("connections:{}:password", connection_id))?;
        entry.delete_password()
    }

    // Similar methods for other credential types...
    pub fn store_ssh_private_key(&self, connection_id: &str, key: &str) -> Result<(), Error>;
    pub fn get_ssh_private_key(&self, connection_id: &str) -> Result<Option<String>, Error>;
    pub fn store_ssl_certificate(&self, connection_id: &str, cert: &str) -> Result<(), Error>;
    pub fn get_ssl_certificate(&self, connection_id: &str) -> Result<Option<String>, Error>;
    pub fn delete_all_credentials(&self, connection_id: &str) -> Result<()>;
}
```

## Connection Manager Service

```rust
pub struct ConnectionManager {
    db: Arc<Mutex<Connection>>,
    credential_manager: Arc<CredentialManager>,
}

impl ConnectionManager {
    pub async fn create_connection(&self, conn: Connection, credentials: SecureCredentials) -> Result<String> {
        // Store in database
        // Store sensitive data in keyring
        // Return connection ID
    }

    pub async fn get_connection(&self, id: &str) -> Result<(Connection, SecureCredentials)> {
        // Get from database
        // Get credentials from keyring
        // Return combined data
    }

    pub async fn update_connection(&self, conn: Connection, credentials: Option<SecureCredentials>) -> Result<()> {
        // Update database
        // Update keyring if credentials provided
    }

    pub async fn delete_connection(&self, id: &str) -> Result<()> {
        // Delete from database
        // Delete all associated keyring entries
    }

    pub async fn list_connections(&self) -> Result<Vec<Connection>> {
        // List all connections from database (without credentials)
    }

    pub async fn test_connection(&self, conn: Connection, credentials: SecureCredentials) -> Result<ConnectionInfo> {
        // Use credentials to test actual database connection
        // Return connection info or error
    }
}
```

## Tauri Commands

```rust
#[tauri::command]
async fn create_connection(
    connection: Connection,
    credentials: SecureCredentials,
    state: State<'_, ConnectionManagerState>
) -> Result<String, String>;

#[tauri::command]
async fn get_connection(
    id: String,
    state: State<'_, ConnectionManagerState>
) -> Result<(Connection, SecureCredentials), String>;

#[tauri::command]
async fn update_connection(
    connection: Connection,
    credentials: Option<SecureCredentials>,
    state: State<'_, ConnectionManagerState>
) -> Result<(), String>;

#[tauri::command]
async fn delete_connection(
    id: String,
    state: State<'_, ConnectionManagerState>
) -> Result<(), String>;

#[tauri::command]
async fn list_connections(
    state: State<'_, ConnectionManagerState>
) -> Result<Vec<Connection>, String>;

#[tauri::command]
async fn test_connection(
    connection: Connection,
    credentials: SecureCredentials,
    state: State<'_, ConnectionManagerState>
) -> Result<ConnectionInfo, String>;
```

## Frontend Integration

### TypeScript Interfaces

```typescript
// Connection metadata (from backend)
interface Connection {
  id: string;
  name: string;
  engine: DatabaseEngine;
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  authType: AuthType;
  sslEnabled: boolean;
  sshTunnelEnabled: boolean;
  sshTunnelHost?: string;
  sshTunnelPort?: number;
  sshTunnelUsername?: string;
  connectionParams: Record<string, any>;
  isFavorite: boolean;
  colorTag?: string;
  createdAt: number;
  updatedAt: number;
  lastConnectedAt?: number;
  connectionCount: number;
}

// Database engines
type DatabaseEngine = 'postgresql' | 'mysql' | 'sqlite' | 'mongodb' | 'redis' | 'elasticsearch' | string;

// Authentication types
type AuthType = 'password' | 'ssh_key' | 'ssl_cert' | 'api_token' | 'windows_auth' | 'kerberos' | 'none';

// Secure credentials (never stored in frontend state)
interface SecureCredentials {
  password?: string;
  sshPrivateKey?: string;
  sshPassphrase?: string;
  sslCertificate?: string;
  sslPrivateKey?: string;
  sslCaCertificate?: string;
  apiToken?: string;
}

// Connection test result
interface ConnectionInfo {
  connected: boolean;
  version?: string;
  databaseName?: string;
  error?: string;
  responseTime?: number;
}
```

### Frontend Service

```typescript
class ConnectionService {
  // Create new connection
  async createConnection(connection: Connection, credentials: SecureCredentials): Promise<string> {
    return await invoke('create_connection', { connection, credentials });
  }

  // Get connection with credentials
  async getConnection(id: string): Promise<{ connection: Connection; credentials: SecureCredentials }> {
    return await invoke('get_connection', { id });
  }

  // Update connection
  async updateConnection(connection: Connection, credentials?: SecureCredentials): Promise<void> {
    return await invoke('update_connection', { connection, credentials });
  }

  // Delete connection
  async deleteConnection(id: string): Promise<void> {
    return await invoke('delete_connection', { id });
  }

  // List all connections (without credentials)
  async listConnections(): Promise<Connection[]> {
    return await invoke('list_connections');
  }

  // Test connection
  async testConnection(connection: Connection, credentials: SecureCredentials): Promise<ConnectionInfo> {
    return await invoke('test_connection', { connection, credentials });
  }
}
```

### Svelte Store Integration

```typescript
import { writable } from 'svelte/store';

interface ConnectionState {
  connections: Connection[];
  loading: boolean;
  error: string | null;
}

export const connectionStore = writable<ConnectionState>({
  connections: [],
  loading: false,
  error: null
});

export const connectionService = new ConnectionService();

// Actions for the store
export const connectionActions = {
  async loadConnections() {
    connectionStore.update(state => ({ ...state, loading: true, error: null }));
    try {
      const connections = await connectionService.listConnections();
      connectionStore.set({ connections, loading: false, error: null });
    } catch (error) {
      connectionStore.update(state => ({ 
        ...state, 
        loading: false, 
        error: error.message 
      }));
    }
  },

  async createConnection(connection: Connection, credentials: SecureCredentials) {
    try {
      const id = await connectionService.createConnection(connection, credentials);
      await this.loadConnections(); // Refresh list
      return id;
    } catch (error) {
      connectionStore.update(state => ({ 
        ...state, 
        error: error.message 
      }));
      throw error;
    }
  },

  async deleteConnection(id: string) {
    try {
      await connectionService.deleteConnection(id);
      await this.loadConnections(); // Refresh list
    } catch (error) {
      connectionStore.update(state => ({ 
        ...state, 
        error: error.message 
      }));
      throw error;
    }
  }
};
```

## Security Considerations

### Backend Security
- **Memory Safety**: Sensitive data wrapped in `SecretString`
- **Keyring Storage**: OS-level encryption for credentials
- **No Logging**: Never log sensitive information
- **Atomic Operations**: Database and keyring operations are atomic
- **Error Handling**: Graceful degradation if keyring unavailable

### Frontend Security
- **Credential Handling**: Credentials never stored in frontend state
- **Secure Transmission**: Use Tauri's secure IPC
- **Input Validation**: Validate all connection parameters
- **Memory Cleanup**: Clear credentials from memory after use

### Validation
- Input validation for all connection parameters
- UUID generation for connection IDs
- Credential format validation
- Connection testing before saving

## Implementation Benefits

### Security
- **Zero Exposure**: Credentials never in plaintext in database
- **OS Protection**: Leverages OS keyring security
- **Memory Safe**: Proper handling of sensitive data
- **Audit Ready**: All operations logged (non-sensitive data)

### Performance
- **Fast Metadata**: SQLite queries for connection lists
- **Secure Caching**: Credentials cached in memory only when needed
- **Efficient Indexing**: Proper database indexes for performance

### Extensibility
- **Multiple Auth Types**: Support for various authentication methods
- **Engine Agnostic**: Works with any database engine
- **Future Sync Ready**: Architecture supports future sync features

This design provides enterprise-grade security while maintaining flexibility and performance for the database management application.
