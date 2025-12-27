import type { Driver } from "./DriverList";

type FieldValues = Record<string, string>;
type Credentials = Record<string, string>;

type DriverConfig = {
  fields: FieldValues;
  credentials: Credentials;
  visibleFields: (keyof FieldValues)[];
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
    },
    credentials: {},
    visibleFields: ["name", "host", "port", "database", "username", "password", "comment"],
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
    credentials: {},
    visibleFields: ["name", "host", "port", "database", "username", "password", "comment"],
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
    credentials: {},
    visibleFields: ["name", "host", "port", "database", "username", "password", "comment"],
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
    credentials: {},
    visibleFields: ["name", "file", "comment"],
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
    credentials: {},
    visibleFields: ["name", "host", "port", "password", "comment"],
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
    credentials: {},
    visibleFields: ["name", "host", "port", "database", "username", "password", "comment"],
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
    credentials: {},
    visibleFields: ["name", "host", "port", "comment"],
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
  },
  credentials: {},
  visibleFields: ["name"],
};

class ConnectionFormStore {
  driver = $state<Driver | null>(null);
  fields = $state<FieldValues>({ ...emptyConfig.fields });
  credentials = $state<Credentials>({});

  get state() {
    return {
      driver: this.driver,
      fields: this.fields,
      credentials: this.credentials,
      visibleFields: this.visibleFields,
    };
  }

  get visibleFields(): (keyof FieldValues)[] {
    if (!this.driver) return emptyConfig.visibleFields;
    const config = driverDefaults[this.driver.id];
    return config ? config.visibleFields : emptyConfig.visibleFields;
  }

  setDriver(driver: Driver | null) {
    this.driver = driver;
    if (driver && driverDefaults[driver.id]) {
      const config = driverDefaults[driver.id];
      this.fields = { ...config.fields, name: driver.name, port: driver.defaultPort ? String(driver.defaultPort) : config.fields.port };
      this.credentials = { ...config.credentials };
    } else {
      this.fields = { ...emptyConfig.fields };
      this.credentials = { ...emptyConfig.credentials };
    }
  }

  updateField(field: keyof FieldValues, value: string) {
    this.fields = { ...this.fields, [field]: value };
  }

  updateCredential(key: keyof Credentials, value: string) {
    this.credentials = { ...this.credentials, [key]: value };
  }

  reset() {
    this.driver = null;
    this.fields = { ...emptyConfig.fields };
    this.credentials = { ...emptyConfig.credentials };
  }
}

export const connectionForm = new ConnectionFormStore();
