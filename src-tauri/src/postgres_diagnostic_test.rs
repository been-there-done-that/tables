
#[cfg(test)]
mod postgres_diag_tests {
    use tokio_postgres::NoTls;
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_postgres_connection_and_hang() {
        // This test replicates the logic in query_commands.rs
        // You can run this with a real connection string to debug:
        // DATABASE_URL=postgres://user:pass@host:port/db cargo test test_postgres_connection_and_hang -- --nocapture
        
        let conn_str = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/postgres".to_string());
        
        println!("Attempting to connect to: {}", conn_str);
        
        let (client, connection) = match tokio_postgres::connect(&conn_str, NoTls).await {
            Ok(res) => res,
            Err(e) => {
                println!("Connection failed: {}", e);
                return;
            }
        };

        println!("Connection created, spawning background task");
        
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Postgres connection error: {}", e);
            }
        });

        println!("Executing simple query...");
        let rows = match client.query("SELECT 1", &[]).await {
            Ok(res) => res,
            Err(e) => {
                println!("Query failed: {}", e);
                return;
            }
        };
        
        println!("Query successful, got {} rows", rows.len());
        assert_eq!(rows.len(), 1);
        
        println!("Executing multi-statement simulation...");
        let queries = vec!["SELECT 1", "SELECT 2"];
        for q in queries {
            println!("Executing: {}", q);
            let res = client.query(q, &[]).await.unwrap();
            println!("Got {} rows", res.len());
        }
        
        println!("All tests passed!");
    }
}
