// Base command interface
export interface CommandResponse<T = any> {
  success: boolean;
  data?: T;
  error?: string;
}

// Theme types
export interface Theme {
  id: string;
  name: string;
  author?: string;
  description?: string;
  theme_data: string;
  is_builtin: boolean;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export type ThemeRecord = Theme;

// Connection types
export interface DatabaseEngine {
  PostgreSQL: 'postgres';
  MySQL: 'mysql';
  SQLite: 'sqlite';
  MongoDB: 'mongodb';
  Redis: 'redis';
  Elasticsearch: 'elasticsearch';
  S3: 's3';
  Athena: 'athena';
  Custom: string;
}

export interface AuthType {
  Password: 'password';
  SshKey: 'ssh_key';
  SslCert: 'ssl_cert';
  ApiToken: 'api_token';
  WindowsAuth: 'windows_auth';
  Kerberos: 'kerberos';
  None: 'none';
  AwsCredentials: 'aws_credentials';
  AwsProfile: 'aws_profile';
  AwsIamRole: 'aws_iam_role';
  AthenaJdbc: 'athena_jdbc';
}

export interface Connection {
  id: string;
  name: string;
  engine: string;
  host?: string;
  port?: number;
  database?: string;
  username?: string;
  auth_type: string;
  ssl_enabled: boolean;
  ssh_tunnel_enabled: boolean;
  ssh_tunnel_host?: string;
  ssh_tunnel_port?: number;
  ssh_tunnel_username?: string;
  connection_params: Record<string, any>;
  is_favorite: boolean;
  color_tag?: string;
  created_at: number;
  updated_at: number;
  last_connected_at?: number;
  connection_count: number;
}

export interface SecureCredentials {
  password?: string;
  ssh_private_key?: string;
  ssh_passphrase?: string;
  ssl_certificate?: string;
  ssl_private_key?: string;
  ssl_ca_certificate?: string;
  api_token?: string;
  aws_access_key_id?: string;
  aws_secret_access_key?: string;
  aws_session_token?: string;
}

export interface ConnectionInfo {
  connected: boolean;
  version?: string;
  database_name?: string;
  error?: string;
  response_time_ms?: number;
}

export interface CreateConnectionRequest {
  connection: Omit<Connection, 'id' | 'created_at' | 'updated_at' | 'last_connected_at' | 'connection_count'>;
  credentials: SecureCredentials;
}

export interface UpdateConnectionRequest {
  id: string;
  connection: Partial<Connection>;
  credentials?: SecureCredentials;
}

// AWS types
export interface AwsProfile {
  name: string;
  access_key_id?: string;
  secret_access_key?: string;
  session_token?: string;
  region?: string;
  profile_source: string;
  is_valid: boolean;
  validation_error?: string;
}

export interface S3Bucket {
  name: string;
  creation_date: string;
  region?: string;
}

export interface S3Object {
  key: string;
  last_modified: string;
  size: number;
  storage_class: string;
  etag: string;
}

export interface S3UploadRequest {
  bucket: string;
  key: string;
  file_path: string;
  content_type?: string;
  metadata?: Record<string, string>;
}

export interface S3DownloadRequest {
  bucket: string;
  key: string;
  local_path: string;
}

// Redis types
export interface RedisInfo {
  redis_version: string;
  redis_mode: string;
  os: string;
  arch_bits: number;
  uptime_in_seconds: number;
  connected_clients: number;
  used_memory: number;
  total_system_memory: number;
  [key: string]: any;
}

export interface RedisDatabase {
  index: number;
  keys: number;
  expires: number;
  avg_ttl?: number;
}

export interface RedisKey {
  key: string;
  type: string;
  size?: number;
  ttl?: number;
  value?: any;
}

export interface RedisCommand {
  command: string;
  args?: any[];
}

// Athena types
export interface AthenaQueryRequest {
  query: string;
  database?: string;
  work_group?: string;
  output_location?: string;
}

export interface AthenaQueryResult {
  query_id: string;
  status: 'SUCCEEDED' | 'FAILED' | 'RUNNING' | 'CANCELLED';
  rows?: any[][];
  column_info?: {
    name: string;
    type: string;
  }[];
  error?: string;
  execution_time_ms?: number;
}

// Plugin types
export interface PluginInfo {
  name: string;
  version: string;
  description: string;
  author: string;
  enabled: boolean;
  commands: string[];
  dependencies: string[];
}

export interface PluginCommand {
  name: string;
  description: string;
  parameters: Record<string, any>;
}

// Common request/response types
export interface SearchRequest {
  query: string;
  limit?: number;
  offset?: number;
}

// System metrics types
export interface SystemMetrics {
  cpu_percent: number;
  threads: number;
  pid: number;
}

export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
  has_more: boolean;
}

// Error types
export interface ApiError {
  code: string;
  message: string;
  details?: any;
}

// Event types
export interface ThemeChangeEvent {
  type: 'current-theme';
  payload: Theme;
}

export interface ConnectionChangeEvent {
  type: 'connection-changed';
  payload: Connection;
}
// Introspection types
export interface MetaDatabase {
  name: string;
  is_connected: boolean;
  is_introspected: boolean;
  is_loading?: boolean; // UI only
  schemas: MetaSchema[];
}

export interface MetaSchema {
  name: string;
  schema_type: "user" | "system";
  tables: MetaTable[];
}

export interface MetaTable {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  table_type: string;
  classification: string;
  last_introspected_at: number;
  columns: MetaColumn[];
  foreign_keys: MetaForeignKey[];
  indexes: MetaIndex[];
}

export interface MetaColumn {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  ordinal_position: number;
  column_name: string;
  raw_type: string;
  logical_type: string;
  nullable: boolean;
  default_value?: string;
  is_primary_key: boolean;
}

export interface MetaForeignKey {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  column_name: string;
  ref_table: string;
  ref_column: string;
}

export interface MetaIndex {
  connection_id: string;
  database: string;
  schema: string;
  table_name: string;
  index_name: string;
  is_unique: boolean;
}
