use std::path::PathBuf;

use radroots_model::types::DatabaseConnection;

use crate::util;

pub struct Radroots {
    pub db: DatabaseConnection,
    pub data_dir: PathBuf,
    pub logs_dir: PathBuf,
}

impl Radroots {
    pub async fn new(data_dir: PathBuf, logs_dir: PathBuf) -> Self {
        util::init_keyring(&data_dir).await;
        let db = util::init_db(&data_dir).await;

        Self {
            db,
            data_dir,
            logs_dir,
        }
    }
}
