export interface Driver {
    id: string;
    name: string;
    icon: string;
    defaultPort?: number;
    provider?: string;  // set for cloud providers; undefined for raw engines
}

export const drivers: Driver[] = [
    { id: 'postgresql',   name: 'PostgreSQL',    icon: 'brand-postgresql',  defaultPort: 5432 },
    { id: 'supabase',     name: 'Supabase',      icon: 'brand-supabase',    defaultPort: 5432, provider: 'supabase' },
    { id: 'neon',         name: 'Neon',          icon: 'database-star',     defaultPort: 5432, provider: 'neon' },
    { id: 'planetscale',  name: 'PlanetScale',   icon: 'brand-planetscale', defaultPort: 5432, provider: 'planetscale' },
    { id: 'mysql',        name: 'MySQL',         icon: 'brand-mysql',       defaultPort: 3306 },
    { id: 'sqlite',       name: 'SQLite',        icon: 'database',          defaultPort: undefined },
    { id: 'mongodb',      name: 'MongoDB',       icon: 'brand-mongodb',     defaultPort: 27017 },
    { id: 'redis',        name: 'Redis',         icon: 'brand-redis',       defaultPort: 6379 },
    { id: 'elasticsearch',name: 'Elasticsearch', icon: 'brand-elastic',     defaultPort: 9200 },
    { id: 's3',           name: 'Amazon S3',     icon: 'brand-aws',         defaultPort: undefined },
    { id: 'athena',       name: 'Amazon Athena', icon: 'brand-aws',         defaultPort: undefined },
    { id: 'custom',       name: 'Custom',        icon: 'database-cog',      defaultPort: undefined },
];
