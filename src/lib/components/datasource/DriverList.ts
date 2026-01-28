import PostgresIcon from '$lib/svg/datasource/PostgresDataSource.png'
import MongoDbIcon from '$lib/svg/datasource/MongoDb.svelte'
import ElasticsearchIcon from '$lib/svg/datasource/Elastic.svelte'
import RedisIcon from '$lib/svg/datasource/Redis.svelte'
import MySQLIcon from '$lib/svg/datasource/MySql.svelte'
import SQLiteIcon from '$lib/svg/datasource/SqliteDatabase.png'

export interface Driver {
  id: string;
  name: string;
  icon: any;
  defaultPort?: number;
  status: 'supported' | 'coming-soon' | 'disabled';
}

export const drivers: Driver[] = [
  { id: 'postgres', name: 'PostgreSQL', icon: PostgresIcon, defaultPort: 5432, status: 'supported' },
  { id: 'mysql', name: 'MySQL', icon: MySQLIcon, defaultPort: 3306, status: 'supported' },
  { id: 'sqlite', name: 'SQLite', icon: SQLiteIcon, defaultPort: undefined, status: 'supported' },
  { id: 'mongodb', name: 'MongoDB', icon: MongoDbIcon, defaultPort: 27017, status: 'supported' },
  { id: 'redis', name: 'Redis', icon: RedisIcon, defaultPort: 6379, status: 'supported' },
  { id: 'elasticsearch', name: 'Elasticsearch', icon: ElasticsearchIcon, defaultPort: 9200, status: 'supported' },
  { id: 'mariadb', name: 'MariaDB', icon: MySQLIcon, defaultPort: 3306, status: 'coming-soon' },
  { id: 'cockroachdb', name: 'CockroachDB', icon: PostgresIcon, defaultPort: 26257, status: 'coming-soon' },
  { id: 'tidb', name: 'TiDB', icon: MySQLIcon, defaultPort: 4000, status: 'disabled' },
];

export const resolveDriverIcon = (id: string) => {
  const driver = drivers.find((d) => d.id === id);
  return (driver ? driver.icon : undefined) as any;
};
