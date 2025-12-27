  import PostgresIcon from '$lib/svg/datasource/Postgres.svelte'
import MongoDbIcon from '$lib/svg/datasource/MongoDb.svelte'
import ElasticsearchIcon from '$lib/svg/datasource/Elastic.svelte'
import RedisIcon from '$lib/svg/datasource/Redis.svelte'
import MySQLIcon from '$lib/svg/datasource/MySql.svelte'
import SQLiteIcon from '$lib/svg/datasource/SQLite.svelte'

export interface Driver {
    id: string;
    name: string;
    icon: typeof PostgresIcon | string; // Component type or fallback string
    defaultPort?: number;
}

export const drivers: Driver[] = [
    { id: 'postgresql', name: 'PostgreSQL', icon: PostgresIcon, defaultPort: 5432 },
    { id: 'mysql', name: 'MySQL', icon: MySQLIcon, defaultPort: 3306 },
    { id: 'sqlite', name: 'SQLite', icon: SQLiteIcon, defaultPort: undefined },
    { id: 'mongodb', name: 'MongoDB', icon: MongoDbIcon, defaultPort: 27017 },
    { id: 'redis', name: 'Redis', icon: RedisIcon, defaultPort: 6379 },
    { id: 'elasticsearch', name: 'Elasticsearch', icon: ElasticsearchIcon, defaultPort: 9200 },
    { id: 's3', name: 'Amazon S3', icon: 'database', defaultPort: undefined },
    { id: 'athena', name: 'Amazon Athena', icon: 'database', defaultPort: undefined },
];
