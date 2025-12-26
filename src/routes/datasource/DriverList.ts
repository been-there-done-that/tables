  import IconPostgresql from '~icons/akar-icons/postgresql-fill'
  import PostgresIcon from '$lib/svg/datasource/Postgres.svelte'

export interface Driver {
    id: string;
    name: string;
    icon: typeof IconPostgresql | string; // Component type or fallback string
    defaultPort?: number;
}

export const drivers: Driver[] = [
    { id: 'postgresql', name: 'PostgreSQL', icon: PostgresIcon, defaultPort: 5432 },
    { id: 'mysql', name: 'MySQL', icon: 'database', defaultPort: 3306 },
    { id: 'sqlite', name: 'SQLite', icon: 'database', defaultPort: undefined },
    { id: 'mongodb', name: 'MongoDB', icon: 'database', defaultPort: 27017 },
    { id: 'redis', name: 'Redis', icon: 'database', defaultPort: 6379 },
    { id: 'elasticsearch', name: 'Elasticsearch', icon: 'database', defaultPort: 9200 },
    { id: 's3', name: 'Amazon S3', icon: 'database', defaultPort: undefined },
    { id: 'athena', name: 'Amazon Athena', icon: 'database', defaultPort: undefined },
    { id: 'custom', name: 'Custom', icon: 'database', defaultPort: undefined },
];
