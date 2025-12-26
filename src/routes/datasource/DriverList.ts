export interface Driver {
    id: string;
    name: string;
    icon: string; // We can use icon names from a library or paths
    defaultPort?: number;
}

export const drivers: Driver[] = [
    { id: 'postgresql', name: 'PostgreSQL', icon: 'brand-postgresql', defaultPort: 5432 },
    { id: 'mysql', name: 'MySQL', icon: 'brand-mysql', defaultPort: 3306 },
    { id: 'sqlite', name: 'SQLite', icon: 'database', defaultPort: undefined },
    { id: 'mongodb', name: 'MongoDB', icon: 'brand-mongodb', defaultPort: 27017 },
    { id: 'redis', name: 'Redis', icon: 'brand-redis', defaultPort: 6379 },
    { id: 'elasticsearch', name: 'Elasticsearch', icon: 'brand-elastic', defaultPort: 9200 },
    { id: 's3', name: 'Amazon S3', icon: 'brand-aws', defaultPort: undefined },
    { id: 'athena', name: 'Amazon Athena', icon: 'brand-aws', defaultPort: undefined },
    { id: 'custom', name: 'Custom', icon: 'database-cog', defaultPort: undefined },
];
