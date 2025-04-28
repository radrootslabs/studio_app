use std::fs::OpenOptions;
use std::path::PathBuf;

use tangle_model::types::DatabaseConnection;
use tangle_model::util::db_conn;

pub async fn init_db(data_dir: &PathBuf) -> DatabaseConnection {
    let mut path = data_dir.clone();
    match std::fs::create_dir_all(path.clone()) {
        Ok(_) => {}
        Err(e) => {
            panic!("Error resolving databse directory {}", e);
        }
    };
    path.push("tangle.db");
    let result = OpenOptions::new().create_new(true).write(true).open(&path);
    match result {
        Ok(_) => println!("Database file created"),
        Err(e) => match e.kind() {
            std::io::ErrorKind::AlreadyExists => println!("Database file exists"),
            _ => {
                panic!("Error creating databse file {}", e);
            }
        },
    }
    let db = db_conn(path).await;
    sqlx::migrate!("./migrations/up").run(&db).await.unwrap();
    db
}

pub async fn init_keyring(data_dir: &PathBuf) -> String {
    let mut path = data_dir.clone();
    match std::fs::create_dir_all(path.clone()) {
        Ok(_) => {}
        Err(err) => {
            panic!("Error resolving databse directory {}", err);
        }
    };
    path.push("keyring_id.txt");
    if path.exists() {
        let keyring_read = match std::fs::read_to_string(&path) {
            Ok(res) => res,
            Err(e) => {
                panic!("Error reading keyring_id.txt {}", e);
            }
        };
        keyring_read.trim().to_string()
    } else {
        let keyring_new = "test-keyring-id".to_string();

        match OpenOptions::new().create_new(true).write(true).open(&path) {
            Ok(_) => match std::fs::write(&path, &keyring_new) {
                Ok(_) => keyring_new,
                Err(e) => {
                    panic!("Error writing keyring_id.txt {}", e);
                }
            },
            Err(e) => {
                panic!("Unexpected error {}", e);
            }
        }
    }
}
