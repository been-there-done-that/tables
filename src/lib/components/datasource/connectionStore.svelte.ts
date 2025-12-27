import type { Driver } from "./DriverList";

type FieldValues = Record<string, string>;
type DriverConfig = {
  fields: FieldValues;
};

const driverDefaults: Record<string, DriverConfig> = {
  postgresql: {
    fields: {
      name: "PostgreSQL",
      host: "localhost",
      port: "5432",
      username: "",
      password: "",
      database: "",
      file: "",
      comment: "",
      ssh_host: "",
      ssh_port: "",
      ssh_user: "",
      ssh_key: "",
      search_path: "",
      application_name: "",
    },
  },
  mysql: {
    fields: {
      name: "MySQL",
      host: "localhost",
      port: "3306",
      username: "",
      password: "",
      database: "",
      file: "",
      comment: "",
    },
  },
  mariadb: {
    fields: {
      name: "MariaDB",
      host: "localhost",
      port: "3306",
      username: "",
      password: "",
      database: "",
      file: "",
      comment: "",
    },
  },
  sqlite: {
    fields: {
      name: "SQLite",
      host: "",
      port: "",
      username: "",
      password: "",
      database: "",
      file: "",
      comment: "",
    },
  },
  redis: {
    fields: {
      name: "Redis",
      host: "localhost",
      port: "6379",
      username: "",
      password: "",
      database: "",
      file: "",
      comment: "",
    },
  },
  mongodb: {
    fields: {
      name: "MongoDB",
      host: "localhost",
      port: "27017",
      username: "",
      password: "",
      database: "",
      file: "",
      comment: "",
    },
  },
  elasticsearch: {
    fields: {
      name: "Elasticsearch",
      host: "localhost",
      port: "9200",
      username: "",
      password: "",
      database: "",
      file: "",
      comment: "",
    },
  },
};

const emptyConfig: DriverConfig = {
  fields: {
    name: "",
    host: "",
    port: "",
    username: "",
    password: "",
    database: "",
    file: "",
    comment: "",
    ssh_host: "",
    ssh_port: "",
    ssh_user: "",
    ssh_key: "",
    search_path: "",
    application_name: "",
  },
};

class ConnectionFormStore {
  driver = $state<Driver | null>(null);
  fields = $state<FieldValues>({ ...emptyConfig.fields });

  get state() {
    return {
      driver: this.driver,
      fields: this.fields,
    };
  }

  setDriver(driver: Driver | null) {
    this.driver = driver;
    if (driver && driverDefaults[driver.id]) {
      const config = driverDefaults[driver.id];
      this.fields = { ...config.fields, name: driver.name, port: driver.defaultPort ? String(driver.defaultPort) : config.fields.port };
    } else {
      this.fields = { ...emptyConfig.fields };
    }
  }

  updateField(field: keyof FieldValues, value: string) {
    this.fields = { ...this.fields, [field]: value };
  }

  reset() {
    this.driver = null;
    this.fields = { ...emptyConfig.fields };
  }
}

export const connectionForm = new ConnectionFormStore();
