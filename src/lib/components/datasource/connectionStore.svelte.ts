import { type Driver, drivers } from "./DriverList";
import { createEmptyConfig, type PostgresConfig, type SqliteConfig } from "$lib/schema/connectionSchema";

import type { ConnectionInfo, Connection } from "$lib/commands/types";

// Union type for all possible config types
type ConnectionConfig = PostgresConfig | SqliteConfig | Record<string, any>;

// Extended config with name field
type NamedConfig = ConnectionConfig & { name: string };

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

  const baseConfig = createEmptyConfig(driver.id as any);
  return { ...baseConfig, name: driver.name } as NamedConfig;
}

class ConnectionFormStore {
  driver = $state<Driver | null>(null);
  fields = $state<NamedConfig>({ version: 1, name: "" } as NamedConfig);
  testResult = $state<ConnectionInfo | null>(null);

  get state() {
    return {
      driver: this.driver,
      fields: this.fields,
      testResult: this.testResult,
    };
  }

  setDriver(driver: Driver | null) {
    // Only reset fields if the driver actually changed
    if (this.driver?.id === driver?.id) {
      return; // Same driver, keep existing data
    }

    this.driver = driver;
    this.testResult = null; // Clear test result when driver changes
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
  }

  setFromConnection(connection: Connection) {
    if (!connection.config_json) return;
    try {
      const config = JSON.parse(connection.config_json);
      this.fields = { ...config, id: connection.id, name: connection.name };
      this.driver = drivers.find(d => d.id === connection.engine) || null;
      this.testResult = null;
    } catch (e) {
      console.error("Failed to parse connection config", e);
    }
  }

  reset() {
    this.driver = null;
    this.fields = { version: 1, name: "" } as NamedConfig;
    this.testResult = null;
  }

  setTestResult(result: ConnectionInfo | null) {
    this.testResult = result;
  }
}

export const connectionForm = new ConnectionFormStore();
