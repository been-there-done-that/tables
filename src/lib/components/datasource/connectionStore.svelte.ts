import type { Driver } from "./DriverList";
import { createEmptyConfig, type PostgresConfig, type SqliteConfig } from "$lib/schema/connectionSchema";

// Union type for all possible config types
type ConnectionConfig = PostgresConfig | SqliteConfig | Record<string, any>;

// Extended config with name field
type NamedConfig = ConnectionConfig & { name: string };

// Get default config for a driver
function getDefaultConfig(driver: Driver): NamedConfig {
  const engineType = driver.id === "postgresql" || driver.id === "mysql" || driver.id === "mariadb"
    ? "postgres"
    : driver.id === "sqlite"
      ? "sqlite"
      : driver.id === "mongodb"
        ? "mongodb"
        : driver.id === "redis"
          ? "redis"
          : driver.id === "elasticsearch"
            ? "elasticsearch"
            : "postgres"; // fallback

  const baseConfig = createEmptyConfig(engineType as any);
  return { ...baseConfig, name: driver.name } as NamedConfig;
}

class ConnectionFormStore {
  driver = $state<Driver | null>(null);
  fields = $state<NamedConfig>({ version: 1, name: "" } as NamedConfig);

  get state() {
    return {
      driver: this.driver,
      fields: this.fields,
    };
  }

  setDriver(driver: Driver | null) {
    // Only reset fields if the driver actually changed
    if (this.driver?.id === driver?.id) {
      return; // Same driver, keep existing data
    }

    this.driver = driver;
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

  reset() {
    this.driver = null;
    this.fields = { version: 1, name: "" } as NamedConfig;
  }
}

export const connectionForm = new ConnectionFormStore();
