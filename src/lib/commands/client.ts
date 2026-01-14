import { invoke } from '@tauri-apps/api/core';
import { emit } from '@tauri-apps/api/event';
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
  SearchRequest,
  SystemMetrics
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
    const response = await this.invokeCommand<string>('create_connection', request);
    if (response.success) {
      await emit('connections-changed');
    }
    return response;
  }

  async getConnection(id: string): Promise<CommandResponse<[Connection, SecureCredentials]>> {
    return this.invokeCommand('get_connection', { id });
  }

  async getConnectionMetadata(id: string): Promise<CommandResponse<ConnectionInfo>> {
    return this.invokeCommand('get_connection_metadata', { id });
  }

  async listConnections(): Promise<CommandResponse<Connection[]>> {
    return this.invokeCommand('list_connections');
  }

  async updateConnection(request: UpdateConnectionRequest): Promise<CommandResponse<void>> {
    const response = await this.invokeCommand<void>('update_connection', request);
    if (response.success) {
      await emit('connections-changed');
    }
    return response;
  }

  async deleteConnection(id: string): Promise<CommandResponse<void>> {
    const response = await this.invokeCommand<void>('delete_connection', { id });
    if (response.success) {
      await emit('connections-changed');
    }
    return response;
  }

  async testConnection(connection: Connection, credentials?: SecureCredentials): Promise<CommandResponse<ConnectionInfo>> {
    return this.invokeCommand('test_connection', { connection, credentials });
  }

  async testConnectionById(id: string): Promise<CommandResponse<ConnectionInfo>> {
    return this.invokeCommand('test_connection_by_id', { id });
  }

  async testConnectionParams(engine: string, config: any): Promise<CommandResponse<ConnectionInfo>> {
    console.debug("[client] testConnectionParams", { engine, config });
    return this.invokeCommand('test_connection_params', {
      params: { engine, config }
    });
  }

  async getFavoriteConnections(): Promise<CommandResponse<Connection[]>> {
    return this.invokeCommand('get_favorite_connections');
  }
  // ...


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

  // System Commands
  async getSystemMetrics(): Promise<CommandResponse<SystemMetrics>> {
    return this.invokeCommand('get_system_metrics');
  }
  // Settings Commands
  async getAppSettings(): Promise<CommandResponse<any>> {
    return this.invokeCommand('get_app_settings');
  }

  async updateAppSetting(key: string, value: string): Promise<CommandResponse<void>> {
    return this.invokeCommand('update_app_setting', { key, value });
  }
}

// Create singleton instance
export const commandClient = new CommandClient();

// Export individual command functions (wrapped to preserve context)
export const getAllThemes = (...args: Parameters<CommandClient["getAllThemes"]>) =>
  commandClient.getAllThemes(...args);
export const getActiveTheme = (...args: Parameters<CommandClient["getActiveTheme"]>) =>
  commandClient.getActiveTheme(...args);
export const setActiveTheme = (...args: Parameters<CommandClient["setActiveTheme"]>) =>
  commandClient.setActiveTheme(...args);
export const createConnection = (...args: Parameters<CommandClient["createConnection"]>) =>
  commandClient.createConnection(...args);
export const getConnection = (...args: Parameters<CommandClient["getConnection"]>) =>
  commandClient.getConnection(...args);
export const getConnectionMetadata = (...args: Parameters<CommandClient["getConnectionMetadata"]>) =>
  commandClient.getConnectionMetadata(...args);
export const listConnections = (...args: Parameters<CommandClient["listConnections"]>) =>
  commandClient.listConnections(...args);
export const updateConnection = (...args: Parameters<CommandClient["updateConnection"]>) =>
  commandClient.updateConnection(...args);
export const deleteConnection = (...args: Parameters<CommandClient["deleteConnection"]>) =>
  commandClient.deleteConnection(...args);
export const testConnection = (...args: Parameters<CommandClient["testConnection"]>) =>
  commandClient.testConnection(...args);
export const testConnectionById = (...args: Parameters<CommandClient["testConnectionById"]>) =>
  commandClient.testConnectionById(...args);
export const testConnectionParams = (...args: Parameters<CommandClient["testConnectionParams"]>) =>
  commandClient.testConnectionParams(...args);
export const getFavoriteConnections = (...args: Parameters<CommandClient["getFavoriteConnections"]>) =>
  commandClient.getFavoriteConnections(...args);
export const searchConnections = (...args: Parameters<CommandClient["searchConnections"]>) =>
  commandClient.searchConnections(...args);
export const updateConnectionStats = (...args: Parameters<CommandClient["updateConnectionStats"]>) =>
  commandClient.updateConnectionStats(...args);
export const checkKeyringAvailable = (...args: Parameters<CommandClient["checkKeyringAvailable"]>) =>
  commandClient.checkKeyringAvailable(...args);
export const getAvailableAwsProfiles = (...args: Parameters<CommandClient["getAvailableAwsProfiles"]>) =>
  commandClient.getAvailableAwsProfiles(...args);
export const getAwsProfileByName = (...args: Parameters<CommandClient["getAwsProfileByName"]>) =>
  commandClient.getAwsProfileByName(...args);
export const testAwsProfile = (...args: Parameters<CommandClient["testAwsProfile"]>) =>
  commandClient.testAwsProfile(...args);
export const listS3Buckets = (...args: Parameters<CommandClient["listS3Buckets"]>) =>
  commandClient.listS3Buckets(...args);
export const listS3Objects = (...args: Parameters<CommandClient["listS3Objects"]>) =>
  commandClient.listS3Objects(...args);
export const uploadS3File = (...args: Parameters<CommandClient["uploadS3File"]>) =>
  commandClient.uploadS3File(...args);
export const downloadS3File = (...args: Parameters<CommandClient["downloadS3File"]>) =>
  commandClient.downloadS3File(...args);
export const deleteS3Object = (...args: Parameters<CommandClient["deleteS3Object"]>) =>
  commandClient.deleteS3Object(...args);
export const getS3BucketInfo = (...args: Parameters<CommandClient["getS3BucketInfo"]>) =>
  commandClient.getS3BucketInfo(...args);
export const getRedisInfo = (...args: Parameters<CommandClient["getRedisInfo"]>) =>
  commandClient.getRedisInfo(...args);
export const listRedisDatabases = (...args: Parameters<CommandClient["listRedisDatabases"]>) =>
  commandClient.listRedisDatabases(...args);
export const listRedisKeys = (...args: Parameters<CommandClient["listRedisKeys"]>) =>
  commandClient.listRedisKeys(...args);
export const getRedisKey = (...args: Parameters<CommandClient["getRedisKey"]>) =>
  commandClient.getRedisKey(...args);
export const executeRedisCommand = (...args: Parameters<CommandClient["executeRedisCommand"]>) =>
  commandClient.executeRedisCommand(...args);
export const deleteRedisKey = (...args: Parameters<CommandClient["deleteRedisKey"]>) =>
  commandClient.deleteRedisKey(...args);
export const executeAthenaQuery = (...args: Parameters<CommandClient["executeAthenaQuery"]>) =>
  commandClient.executeAthenaQuery(...args);
export const getAvailablePlugins = (...args: Parameters<CommandClient["getAvailablePlugins"]>) =>
  commandClient.getAvailablePlugins(...args);
export const enablePlugin = (...args: Parameters<CommandClient["enablePlugin"]>) =>
  commandClient.enablePlugin(...args);
export const disablePlugin = (...args: Parameters<CommandClient["disablePlugin"]>) =>
  commandClient.disablePlugin(...args);
export const getSystemMetrics = (...args: Parameters<CommandClient["getSystemMetrics"]>) =>
  commandClient.getSystemMetrics(...args);
export const getPluginInfo = (...args: Parameters<CommandClient["getPluginInfo"]>) =>
  commandClient.getPluginInfo(...args);
export const initializeAllPlugins = (...args: Parameters<CommandClient["initializeAllPlugins"]>) =>
  commandClient.initializeAllPlugins(...args);
