// Jackson Coxson

use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sqlite::State;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedSender},
    oneshot::{self, Sender},
};

#[derive(Clone)]
pub struct Database {
    sender: UnboundedSender<(DatabaseAction, Sender<DatabaseReturn>)>,
    pub behind_traefik: bool,
    pub base_url: String,
}

#[derive(Debug)]
pub enum DatabaseAction {
    GetUrl(String),
    InsertUrl((String, String)),
    RemoveUrl(String),
    ModifyUrl((String, String)),
    ModifyComment((String, String)),
    Log((String, String, String)),
    GetStats,
    GetLogs(String),
}

#[derive(Debug)]
pub enum DatabaseReturn {
    GetUrl(String),
    InsertUrl,
    RemoveUrl,
    ModifyUrl,
    ModifyComment,
    Log,
    GetStats(Vec<DatabaseStats>),
    GetLogs(Vec<DatabaseLog>),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseStats {
    url: String,
    code: String,
    comment: String,
    visits: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseLog {
    timestamp: String,
    ip: String,
    url: String,
}

impl Database {
    pub fn new() -> Self {
        let filename = std::env::var("SQLITE_PATH").unwrap_or("riplakish.db".to_string());
        let connection = sqlite::open(filename).expect("Failed to read to database");

        // Make sure the tables exist
        // log, redirects
        info!("Checking for required tables");
        let query = "SELECT name FROM sqlite_master WHERE type='table' AND name='log';";
        let mut exists = false;
        connection
            .iterate(query, |_| {
                exists = true;
                true
            })
            .expect("Unable to check table");
        if !exists {
            let query =
                "CREATE TABLE log (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp DATETIME, redirect TEXT, url TEXT, ip TEXT);";
            connection.execute(query).unwrap();
        }
        let query = "SELECT name FROM sqlite_master WHERE type='table' AND name='redirects';";
        let mut exists = false;
        connection
            .iterate(query, |_| {
                exists = true;
                true
            })
            .expect("Unable to insert table");
        if !exists {
            let query =
                "CREATE TABLE redirects (id INTEGER PRIMARY KEY AUTOINCREMENT, url TEXT, redirect TEXT, comment TEXT);";
            connection.execute(query).unwrap();
        }

        let (tx, mut rx) = unbounded_channel::<(DatabaseAction, Sender<DatabaseReturn>)>();

        tokio::spawn(async move {
            loop {
                while let Some((instruction, return_channel)) = rx.recv().await {
                    match instruction {
                        DatabaseAction::GetUrl(code) => {
                            let query = "SELECT * FROM redirects WHERE redirect = ?";
                            if let Ok(mut statement) = connection.prepare(query) {
                                if statement.bind((1, code.as_str())).is_ok() {
                                    let mut res = None;
                                    while let Ok(State::Row) = statement.next() {
                                        if let Ok(s) = statement.read::<String, _>("url") {
                                            res = Some(DatabaseReturn::GetUrl(s));
                                        } else {
                                            error!("Could not read statement as a string");
                                        }
                                    }
                                    if let Some(res) = res {
                                        if return_channel.send(res).is_err() {
                                            error!(
                                                "Return channel closed before response was sent"
                                            );
                                        }
                                    } else {
                                        warn!("Not found in database");
                                    }
                                } else {
                                    error!("Unable to bind parameter")
                                }
                            } else {
                                error!("Unable to prepare query???");
                            }
                        }
                        DatabaseAction::InsertUrl((url, code)) => {
                            let query = format!(
                                "INSERT INTO redirects (url, redirect) VALUES ('{url}', '{code}');"
                            );

                            if let Err(e) = connection.execute(query) {
                                error!("Failed to insert URL: {:?}", e);
                            } else if return_channel.send(DatabaseReturn::InsertUrl).is_err() {
                                warn!("Return channel closed before response was sent");
                            }
                        }
                        DatabaseAction::RemoveUrl(code) => {
                            let query = format!("DELETE FROM redirects WHERE redirect = '{code}';");
                            if let Err(e) = connection.execute(query) {
                                error!("Failed to remove URL: {:?}", e);
                            } else if return_channel.send(DatabaseReturn::RemoveUrl).is_err() {
                                warn!("Return channel closed before response was sent");
                            }
                        }
                        DatabaseAction::ModifyUrl((code, new_url)) => {
                            let query = format!(
                                "UPDATE redirects SET url = '{new_url}' WHERE redirect = '{code}';"
                            );
                            if let Err(e) = connection.execute(query) {
                                error!("Failed to modify URL: {:?}", e);
                            } else if return_channel.send(DatabaseReturn::ModifyUrl).is_err() {
                                warn!("Return channel closed before response was sent");
                            }
                        }
                        DatabaseAction::ModifyComment((code, new_comment)) => {
                            let query = "UPDATE redirects SET comment = ? WHERE redirect = ?;";
                            let mut statement = connection.prepare(query).unwrap();
                            statement
                                .bind(&[(1, new_comment.as_str()), (2, code.as_str())][..])
                                .unwrap();
                            if let Err(e) = statement.next() {
                                error!("Failed to modify URL: {:?}", e);
                            } else if return_channel.send(DatabaseReturn::ModifyComment).is_err() {
                                warn!("Return channel closed before response was sent");
                            }
                        }
                        DatabaseAction::Log((code, url, ip)) => {
                            let naive_date_time =
                                chrono::offset::Local::now().format("%m/%d/%Y %T");
                            let query = format!(
                                "INSERT INTO log (redirect, ip, url, timestamp) VALUES ('{code}', '{ip}', '{url}', '{naive_date_time}');"
                            );
                            if let Err(e) = connection.execute(query) {
                                error!("Failed to log request: {:?}", e);
                            } else if return_channel.send(DatabaseReturn::Log).is_err() {
                                warn!("Return channel closed before response was sent");
                            }
                        }
                        DatabaseAction::GetStats => {
                            let query =
                                "SELECT r.url, r.redirect, COUNT(l.id) AS log_count, r.comment
                                                FROM redirects r
                                                LEFT JOIN log l ON r.redirect = l.redirect
                                                GROUP BY r.url, r.redirect;";
                            let mut statement = connection.prepare(query).unwrap();
                            let mut res = Vec::new();
                            while let Ok(State::Row) = statement.next() {
                                if let Ok(url) = statement.read::<String, _>(0) {
                                    if let Ok(code) = statement.read::<String, _>(1) {
                                        if let Ok(clicks) = statement.read::<i64, _>(2) {
                                            if let Ok(comment) =
                                                statement.read::<Option<String>, _>(3)
                                            {
                                                res.push(DatabaseStats {
                                                    url,
                                                    code,
                                                    comment: comment.unwrap_or("".to_string()),
                                                    visits: clicks as usize,
                                                });
                                            } else {
                                                error!("Failed to read comment!");
                                            }
                                        } else {
                                            error!("Could not read clicks as a number");
                                        }
                                    } else {
                                        error!("Could not read code as a string");
                                    }
                                } else {
                                    error!("Could not read url as a string");
                                }
                            }
                            if return_channel.send(DatabaseReturn::GetStats(res)).is_err() {
                                warn!("Return channel closed before response was sent");
                            }
                        }
                        DatabaseAction::GetLogs(code) => {
                            let mut res = Vec::new();
                            let mut statement = connection
                                .prepare("SELECT * FROM log WHERE redirect = ?")
                                .unwrap();
                            statement.bind((1, code.as_str())).unwrap();
                            while let Ok(State::Row) = statement.next() {
                                if let Ok(timestamp) = statement.read::<String, _>(1) {
                                    if let Ok(ip) = statement.read::<String, _>(4) {
                                        if let Ok(url) = statement.read::<String, _>(3) {
                                            res.push(DatabaseLog { url, timestamp, ip });
                                        } else {
                                            error!("Failed to read URL from log!");
                                        }
                                    } else {
                                        error!("Failed to read IP from log!");
                                    }
                                } else {
                                    error!("Failed to read timestamp from log!");
                                }
                            }
                            if return_channel.send(DatabaseReturn::GetLogs(res)).is_err() {
                                warn!("Return channel closed before response was sent");
                            }
                        }
                    }
                }
            }
        });
        let behind_traefik = if let Ok(v) = std::env::var("BEHIND_TRAEFIK") {
            v == "true"
        } else {
            false
        };

        let base_url = std::env::var("BASE_URL").expect("Base URL is not set");

        Self {
            sender: tx,
            behind_traefik,
            base_url,
        }
    }

    // Getters haha just like Java

    pub async fn get_url(&self, code: String) -> Option<String> {
        if check_string_injection(&code) {
            warn!("Request failed injection test: {code}");
            return None;
        }
        let request = DatabaseAction::GetUrl(code.to_string());
        let (tx, rx) = oneshot::channel();
        if self.sender.send((request, tx)).is_ok() {
            if let Ok(DatabaseReturn::GetUrl(url)) = rx.await {
                return Some(url);
            }
        }
        None
    }

    pub async fn insert_url(&self, url: &str, code: &str) -> bool {
        if check_string_injection(url) || check_string_injection(code) {
            warn!("Request failed injection test: {url} {code}");
            return false;
        }
        let request = DatabaseAction::InsertUrl((url.to_string(), code.to_string()));
        let (tx, rx) = oneshot::channel();
        if self.sender.send((request, tx)).is_ok() {
            if let Ok(DatabaseReturn::InsertUrl) = rx.await {
                return true;
            }
        }
        false
    }

    pub async fn remove_url(&self, url: String) -> bool {
        if check_string_injection(&url) {
            warn!("Request failed injection test: {url}");
            return false;
        }
        let request = DatabaseAction::RemoveUrl(url);
        let (tx, rx) = oneshot::channel();
        if self.sender.send((request, tx)).is_ok() {
            if let Ok(DatabaseReturn::RemoveUrl) = rx.await {
                return true;
            }
        }
        false
    }

    pub async fn modify_url(&self, code: String, url: String) -> bool {
        if check_string_injection(&code) || check_string_injection(&url) {
            warn!("Request failed injection test: {code} {url}");
            return false;
        }
        let request = DatabaseAction::ModifyUrl((code, url));
        let (tx, rx) = oneshot::channel();
        if self.sender.send((request, tx)).is_ok() {
            if let Ok(DatabaseReturn::ModifyUrl) = rx.await {
                return true;
            }
        }
        false
    }

    pub async fn modify_comment(&self, code: String, comment: String) -> bool {
        if check_string_injection(&code) {
            warn!("Request failed injection test: {code} {comment}");
            return false;
        }
        let request = DatabaseAction::ModifyComment((code, comment));
        let (tx, rx) = oneshot::channel();
        if self.sender.send((request, tx)).is_ok() {
            if let Ok(DatabaseReturn::ModifyComment) = rx.await {
                return true;
            }
        }
        false
    }

    pub async fn log(&self, code: String, url: String, ip: String) -> bool {
        info!("{ip} visited {code}");
        if check_string_injection(&code) || check_string_injection(&ip) {
            warn!("Request failed injection test: {code} {ip}");
            return false;
        }
        let request = DatabaseAction::Log((code, url, ip));
        let (tx, rx) = oneshot::channel();
        if self.sender.send((request, tx)).is_ok() {
            if let Ok(DatabaseReturn::Log) = rx.await {
                return true;
            }
        }
        false
    }

    pub async fn get_stats(&self) -> Vec<DatabaseStats> {
        let (tx, rx) = oneshot::channel();
        if self.sender.send((DatabaseAction::GetStats, tx)).is_ok() {
            if let Ok(DatabaseReturn::GetStats(stats)) = rx.await {
                return stats;
            }
        }
        Vec::new()
    }

    pub async fn get_logs(&self, code: String) -> Vec<DatabaseLog> {
        let (tx, rx) = oneshot::channel();
        if self
            .sender
            .send((DatabaseAction::GetLogs(code), tx))
            .is_ok()
        {
            if let Ok(DatabaseReturn::GetLogs(logs)) = rx.await {
                return logs;
            }
        }
        Vec::new()
    }
}

fn check_string_injection(s: &str) -> bool {
    for c in s.chars() {
        if !c.is_alphanumeric() {
            match c {
                '.' | ':' | '/' | '-' | '_' => continue,
                _ => return true,
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn f1() {
        dotenv::dotenv().ok();
        let db = Database::new();
        assert!(db.insert_url("https://google.com", "asdf").await);
        assert!(db.get_url("asdf".to_string()).await == Some("https://google.com".to_string()))
    }

    #[tokio::test]
    async fn log() {
        dotenv::dotenv().ok();
        let db = Database::new();
        assert!(
            db.log(
                "asdf".to_string(),
                "google.com".to_string(),
                "127.0.0.1".to_string()
            )
            .await
        );
    }

    #[tokio::test]
    async fn stats() {
        dotenv::dotenv().ok();
        let db = Database::new();
        assert!(!db.get_stats().await.is_empty());
    }
}
