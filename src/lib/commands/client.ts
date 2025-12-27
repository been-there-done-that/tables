import { invoke } from '@tauri-apps/api/core';
import type {
  CommandResponse,
  Theme,
  Connection,
  CreateConnectionRequest,
  UpdateConnectionRequest,
  SecureCredentials,
  ConnectionInfo,
  AwsProfile,
  S3Bucket,
  S3Object,
  S3UploadRequest,
  S3DownloadRequest,
  RedisInfo,
  RedisDatabase,
  RedisKey,
  RedisCommand,
  AthenaQueryRequest,
  AthenaQueryResult,
  PluginInfo,
  SearchRequest
} from './types';

/**
 * Type-safe Tauri command client with error handling
 */
export class CommandClient {
  /**
   * Generic command invoker with type safety and error handling
   */
  private async invokeCommand<T>(
    command: string,
    args?: Record<string, any>
  ): Promise<CommandResponse<T>> {
    try {
      const result = await invoke<T>(command, args);
      return {
        success: true,
        data: result
      };
    } catch (error) {
      return {
        success: false,
        error: error instanceof Error ? error.message : String(error)
      };
    }
  }

  // Theme Commands
  async getAllThemes(): Promise<CommandResponse<Theme[]>> {
    return this.invokeCommand('get_all_themes');
  }

  async getActiveTheme(): Promise<CommandResponse<Theme | null>> {
    return this.invokeCommand('get_active_theme');
  }

  async setActiveTheme(themeId: string): Promise<CommandResponse<void>> {
    return this.invokeCommand('set_active_theme', { themeId });
  }

  // Connection Commands
  async createConnection(request: CreateConnectionRequest): Promise<CommandResponse<string>> {
    return this.invokeCommand('create_connection', request);
  }

  async getConnection(id: string): Promise<CommandResponse<Connection>> {
    return this.invokeCommand('get_connection', { id });
  }

  async getConnectionMetadata(id: string): Promise<CommandResponse<ConnectionInfo>> {
    return this.invokeCommand('get_connection_metadata', { id });
  }

  async listConnections(): Promise<CommandResponse<Connection[]>> {
    return this.invokeCommand('list_connections');
  }

  async updateConnection(request: UpdateConnectionRequest): Promise<CommandResponse<void>> {
    return this.invokeCommand('update_connection', request);
  }

  async deleteConnection(id: string): Promise<CommandResponse<void>> {
    return this.invokeCommand('delete_connection', { id });
  }

  async testConnection(connection: Connection, credentials?: SecureCredentials): Promise<CommandResponse<ConnectionInfo>> {
    return this.invokeCommand('test_connection', { connection, credentials });
  }

  async testConnectionParams(engine: string, config: any): Promise<CommandResponse<ConnectionInfo>> {
    return this.invokeCommand('test_connection_params', {
      params: { engine, config }
    });
  }

  async getFavoriteConnections(): Promise<CommandResponse<Connection[]>> {
    return this.invokeCommand('get_favorite_connections');
  }

  async searchConnections(request: SearchRequest): Promise<CommandResponse<Connection[]>> {
    return this.invokeCommand('search_connections', request);
  }

  async updateConnectionStats(id: string): Promise<CommandResponse<void>> {
    return this.invokeCommand('update_connection_stats', { id });
  }

  async checkKeyringAvailable(): Promise<CommandResponse<boolean>> {
    return this.invokeCommand('check_keyring_available');
  }

  // AWS Commands
  async getAvailableAwsProfiles(): Promise<CommandResponse<AwsProfile[]>> {
    return this.invokeCommand('get_available_aws_profiles');
  }

  async getAwsProfileByName(name: string): Promise<CommandResponse<AwsProfile>> {
    return this.invokeCommand('get_aws_profile_by_name', { name });
  }

  async testAwsProfile(profile: AwsProfile): Promise<CommandResponse<boolean>> {
    return this.invokeCommand('test_aws_profile', { profile });
  }

  async listS3Buckets(profile?: string): Promise<CommandResponse<S3Bucket[]>> {
    return this.invokeCommand('list_s3_buckets', { profile });
  }

  async listS3Objects(bucket: string, prefix?: string, profile?: string): Promise<CommandResponse<S3Object[]>> {
    return this.invokeCommand('list_s3_objects', { bucket, prefix, profile });
  }

  async uploadS3File(request: S3UploadRequest): Promise<CommandResponse<void>> {
    return this.invokeCommand('upload_s3_file', request);
  }

  async downloadS3File(request: S3DownloadRequest): Promise<CommandResponse<void>> {
    return this.invokeCommand('download_s3_file', request);
  }

  async deleteS3Object(bucket: string, key: string, profile?: string): Promise<CommandResponse<void>> {
    return this.invokeCommand('delete_s3_object', { bucket, key, profile });
  }

  async getS3BucketInfo(bucket: string, profile?: string): Promise<CommandResponse<any>> {
    return this.invokeCommand('get_s3_bucket_info', { bucket, profile });
  }

  // Redis Commands
  async getRedisInfo(connectionId: string): Promise<CommandResponse<RedisInfo>> {
    return this.invokeCommand('get_redis_info', { connectionId });
  }

  async listRedisDatabases(connectionId: string): Promise<CommandResponse<RedisDatabase[]>> {
    return this.invokeCommand('list_redis_databases', { connectionId });
  }

  async listRedisKeys(connectionId: string, database?: number, pattern?: string): Promise<CommandResponse<RedisKey[]>> {
    return this.invokeCommand('list_redis_keys', { connectionId, database, pattern });
  }

  async getRedisKey(connectionId: string, key: string, database?: number): Promise<CommandResponse<RedisKey>> {
    return this.invokeCommand('get_redis_key', { connectionId, key, database });
  }

  async executeRedisCommand(connectionId: string, command: RedisCommand): Promise<CommandResponse<any>> {
    return this.invokeCommand('execute_redis_command', { connectionId, command });
  }

  async deleteRedisKey(connectionId: string, key: string, database?: number): Promise<CommandResponse<void>> {
    return this.invokeCommand('delete_redis_key', { connectionId, key, database });
  }

  // Athena Commands
  async executeAthenaQuery(request: AthenaQueryRequest): Promise<CommandResponse<AthenaQueryResult>> {
    return this.invokeCommand('execute_athena_query', request);
  }

  // Plugin Commands
  async getAvailablePlugins(): Promise<CommandResponse<PluginInfo[]>> {
    return this.invokeCommand('get_available_plugins');
  }

  async enablePlugin(name: string): Promise<CommandResponse<void>> {
    return this.invokeCommand('enable_plugin', { name });
  }

  async disablePlugin(name: string): Promise<CommandResponse<void>> {
    return this.invokeCommand('disable_plugin', { name });
  }

  async getPluginInfo(name: string): Promise<CommandResponse<PluginInfo>> {
    return this.invokeCommand('get_plugin_info', { name });
  }

  async initializeAllPlugins(): Promise<CommandResponse<void>> {
    return this.invokeCommand('initialize_all_plugins');
  }
}

// Create singleton instance
export const commandClient = new CommandClient();

// Export individual command functions for convenience
export const {
  getAllThemes,
  getActiveTheme,
  setActiveTheme,
  createConnection,
  getConnection,
  getConnectionMetadata,
  listConnections,
  updateConnection,
  deleteConnection,
  testConnection,
  getFavoriteConnections,
  searchConnections,
  updateConnectionStats,
  checkKeyringAvailable,
  getAvailableAwsProfiles,
  getAwsProfileByName,
  testAwsProfile,
  listS3Buckets,
  listS3Objects,
  uploadS3File,
  downloadS3File,
  deleteS3Object,
  getS3BucketInfo,
  getRedisInfo,
  listRedisDatabases,
  listRedisKeys,
  getRedisKey,
  executeRedisCommand,
  deleteRedisKey,
  executeAthenaQuery,
  testConnectionParams,
  getAvailablePlugins,
  enablePlugin,
  disablePlugin,
  getPluginInfo,
  initializeAllPlugins
} = commandClient;
