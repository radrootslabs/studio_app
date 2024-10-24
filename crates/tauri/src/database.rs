use sqlx::sqlite::SqlitePoolOptions;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub async fn setup_db(data_dir: &PathBuf) -> sqlx::Pool<sqlx::Sqlite> {
    let mut path = data_dir.clone();
    match std::fs::create_dir_all(path.clone()) {
        Ok(_) => {}
        Err(err) => {
            panic!("Error resolving databse directory {}", err);
        }
    };
    path.push("radroots_db.sqlite");
    let result = OpenOptions::new().create_new(true).write(true).open(&path);
    match result {
        Ok(_) => println!("Database file created"),
        Err(err) => match err.kind() {
            std::io::ErrorKind::AlreadyExists => println!("Database file exists"),
            _ => {
                panic!("Error creating databse file {}", err);
            }
        },
    }
    let db = SqlitePoolOptions::new()
        .connect(path.to_str().unwrap())
        .await
        .unwrap();
    sqlx::migrate!("./migrations").run(&db).await.unwrap();
    db
}
