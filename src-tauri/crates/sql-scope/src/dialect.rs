use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dialect {
    Postgres,
    Sqlite,
    Mysql,
}

impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Dialect::Postgres => write!(f, "postgres"),
            Dialect::Sqlite => write!(f, "sqlite"),
            Dialect::Mysql => write!(f, "mysql"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_postgres() {
        assert_eq!(Dialect::Postgres.to_string(), "postgres");
    }

    #[test]
    fn display_sqlite() {
        assert_eq!(Dialect::Sqlite.to_string(), "sqlite");
    }

    #[test]
    fn display_mysql() {
        assert_eq!(Dialect::Mysql.to_string(), "mysql");
    }

    #[test]
    fn dialect_hash_in_hashmap() {
        use std::collections::HashMap;
        let mut map: HashMap<Dialect, &str> = HashMap::new();
        map.insert(Dialect::Postgres, "pg");
        map.insert(Dialect::Sqlite, "sq");
        map.insert(Dialect::Mysql, "my");
        assert_eq!(map[&Dialect::Postgres], "pg");
        assert_eq!(map[&Dialect::Sqlite], "sq");
        assert_eq!(map[&Dialect::Mysql], "my");
    }
}
