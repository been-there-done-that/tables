import { type Driver, drivers } from "./DriverList";
import { createEmptyConfig, type PostgresConfig, type SqliteConfig } from "$lib/schema/connectionSchema";

import type { ConnectionInfo, Connection } from "$lib/commands/types";

// Union type for all possible config types
type ConnectionConfig = PostgresConfig | SqliteConfig | Record<string, any>;

// Extended config with name field
type NamedConfig = ConnectionConfig & { name: string; id?: string };

type FormStatus = {
  type: 'idle' | 'testing' | 'success' | 'error';
  message: string;
  details?: string;
};

// Get default config for a driver
function getDefaultConfig(driver: Driver): NamedConfig {
  // const engineType = driver.id === "postgresql" || driver.id === "mysql" || driver.id === "mariadb"
  // ? "postgres"
  // : driver.id === "sqlite"
  //   ? "sqlite"
  //   : driver.id === "mongodb"
  //     ? "mongodb"
  //     : driver.id === "redis"
  //       ? "redis"
  //       : driver.id === "elasticsearch"
  //         ? "elasticsearch"
  //         : "postgres"; // fallback

  const engineId = driver.provider ? "postgres" : driver.id;
  const baseConfig = createEmptyConfig(engineId as any);
  return { ...baseConfig, name: driver.name } as NamedConfig;
}

class ConnectionFormStore {
  driver = $state<Driver | null>(null);
  fields = $state<NamedConfig>({ version: 1, name: "" } as NamedConfig);
  testResult = $state<ConnectionInfo | null>(null);
  status = $state<FormStatus>({ type: 'idle', message: '' });

  get state() {
    return {
      driver: this.driver,
      fields: this.fields,
      testResult: this.testResult,
      status: this.status,
    };
  }

  setDriver(driver: Driver | null) {
    // Only reset fields if the driver actually changed
    if (this.driver?.id === driver?.id) {
      return; // Same driver, keep existing data
    }

    this.driver = driver;
    this.testResult = null; // Clear test result when driver changes
    this.status = { type: 'idle', message: '' }; // Clear status
    if (driver) {
      this.fields = getDefaultConfig(driver);
    } else {
      this.fields = { version: 1, name: "" } as NamedConfig;
    }
  }

  updateField(path: string, value: any) {
    // Handle nested paths like "db.host" or "transport.type"
    const keys = path.split(".");
    const newFields = { ...this.fields } as any;
    let current = newFields;

    for (let i = 0; i < keys.length - 1; i++) {
      const key = keys[i];
      if (!current[key] || typeof current[key] !== 'object') {
        current[key] = {};
      } else {
        current[key] = { ...current[key] };
      }
      current = current[key];
    }

    current[keys[keys.length - 1]] = value;
    this.fields = newFields;

    // Reset status if it was success or error
    if (this.status.type === 'success' || this.status.type === 'error') {
      this.status = { type: 'idle', message: '' };
    }
  }

  setFromConnection(connection: Connection) {
    try {
      let config: any = null;

      // Prioritize config_json as it contains the full nested structure
      if (connection.config_json) {
        try {
          config = JSON.parse(connection.config_json);
        } catch (e) {
          console.warn("Failed to parse config_json, falling back to connection_params", e);
        }
      }

      // Fallback to connection_params if config_json failed or is missing
      if (!config && connection.connection_params) {
        config = connection.connection_params;
      }

      if (config) {
        // Ensure we preserve the ID and Name from the connection record
        this.fields = {
          ...config,
          id: connection.id,
          name: connection.name
        };
        const providerLabel = connection.provider ?? null;
        this.driver = (
          providerLabel
            ? drivers.find(d => d.provider === providerLabel)
            : drivers.find(d => d.id === connection.engine && !d.provider)
        ) || null;
        this.testResult = null;
        this.status = { type: 'idle', message: '' };

        console.debug("[ConnectionFormStore] Loaded connection:", {
          id: this.fields.id,
          name: this.fields.name,
          driver: this.driver?.id,
          fieldsStructure: Object.keys(this.fields)
        });
      } else {
        console.error("No valid configuration found in connection record", connection);
      }
    } catch (e) {
      console.error("Critical error in setFromConnection", e);
    }
  }

  reset() {
    this.driver = null;
    this.fields = { version: 1, name: "" } as NamedConfig;
    this.testResult = null;
    this.status = { type: 'idle', message: '' };
  }

  setStatus(status: FormStatus) {
    this.status = status;
  }

  setTestResult(result: ConnectionInfo | null) {
    this.testResult = result;
  }
}

export const connectionForm = new ConnectionFormStore();
