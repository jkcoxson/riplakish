// Jackson Coxson

use log::{error, info, warn};
use serde::{Deserialize, Serialize};
use sqlite::State;

#[derive(Clone)]
pub struct Database {
    pub behind_traefik: bool,
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub filename: String,
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
        let username = std::env::var("USERNAME").expect("USERNAME environment variable not set");
        let password = std::env::var("PASSWORD").expect("PASSWORD environment variable not set");
        let filename = std::env::var("SQLITE_PATH").unwrap_or("riplakish.db".to_string());
        let connection = sqlite::open(&filename).expect("Failed to read to database");

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
        let query = "SELECT name FROM sqlite_master WHERE type='table' AND name='tokens';";
        let mut exists = false;
        connection
            .iterate(query, |_| {
                exists = true;
                true
            })
            .expect("Unable to insert table");
        if !exists {
            let query =
                "CREATE TABLE tokens (id INTEGER PRIMARY KEY AUTOINCREMENT, token TEXT, expiration DATETIME);";
            connection.execute(query).unwrap();
        }

        let behind_traefik = if let Ok(v) = std::env::var("BEHIND_TRAEFIK") {
            v == "true"
        } else {
            false
        };

        let base_url = std::env::var("BASE_URL").expect("Base URL is not set");

        Self {
            behind_traefik,
            base_url,
            username,
            password,
            filename,
        }
    }

    // Getters haha just like Java

    pub async fn get_url(&self, code: String) -> Option<String> {
        if check_string_injection(&code) {
            warn!("Request failed injection test: {code}");
            return None;
        }
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return None;
            }
        };

        let query = "SELECT * FROM redirects WHERE redirect = ?";
        let mut statement = match connection.prepare(query) {
            Ok(stmt) => stmt,
            Err(err) => {
                error!("Failed to prepare query: {:?}", err);
                return None;
            }
        };

        if let Err(err) = statement.bind((1, code.as_str())) {
            error!("Failed to bind parameter: {:?}", err);
            return None;
        }

        let mut res = None;
        while let Ok(State::Row) = statement.next() {
            if let Ok(s) = statement.read::<String, _>("url") {
                res = Some(s);
            } else {
                error!("Could not read statement as a string");
            }
        }

        if let Some(res) = res {
            Some(res)
        } else {
            warn!("Not found in database");
            None
        }
    }

    pub async fn insert_url(&self, url: &str, code: &str) -> bool {
        if check_string_injection(url) || check_string_injection(code) {
            warn!("Request failed injection test: {url} {code}");
            return false;
        }
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return false;
            }
        };

        let query = format!("INSERT INTO redirects (url, redirect) VALUES ('{url}', '{code}');");
        if let Err(err) = connection.execute(query) {
            error!("Failed to insert URL: {:?}", err);
            false
        } else {
            true
        }
    }

    pub async fn remove_url(&self, code: String) -> bool {
        if check_string_injection(&code) {
            warn!("Request failed injection test: {code}");
            return false;
        }
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return false;
            }
        };

        let query = format!("DELETE FROM redirects WHERE redirect = '{code}';");
        if let Err(err) = connection.execute(query) {
            error!("Failed to remove code: {:?}", err);
            false
        } else {
            true
        }
    }

    pub async fn modify_url(&self, code: String, url: String) -> bool {
        if check_string_injection(&code) || check_string_injection(&url) {
            warn!("Request failed injection test: {code} {url}");
            return false;
        }
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return false;
            }
        };

        let query = format!("UPDATE redirects SET url = '{url}' WHERE redirect = '{code}';");
        if let Err(err) = connection.execute(query) {
            error!("Failed to modify URL: {:?}", err);
            false
        } else {
            true
        }
    }

    pub async fn modify_comment(&self, code: String, comment: String) -> bool {
        if check_string_injection(&code) {
            warn!("Request failed injection test: {code} {comment}");
            return false;
        }
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return false;
            }
        };

        let query = "UPDATE redirects SET comment = ? WHERE redirect = ?;";
        let mut statement = match connection.prepare(query) {
            Ok(stmt) => stmt,
            Err(err) => {
                error!("Failed to prepare query: {:?}", err);
                return false;
            }
        };

        if let Err(err) = statement.bind(&[(1, comment.as_str()), (2, code.as_str())][..]) {
            error!("Failed to bind parameters: {:?}", err);
            return false;
        }

        if let Err(err) = statement.next() {
            error!("Failed to modify URL: {:?}", err);
            false
        } else {
            true
        }
    }

    pub async fn log(&self, code: String, url: String, ip: String) -> bool {
        info!("{ip} visited {code}");
        if check_string_injection(&code) || check_string_injection(&ip) {
            warn!("Request failed injection test: {code} {ip}");
            return false;
        }
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return false;
            }
        };

        let naive_date_time = chrono::offset::Local::now().format("%m/%d/%Y %T");
        let query = format!("INSERT INTO log (redirect, ip, url, timestamp) VALUES ('{code}', '{ip}', '{url}', '{naive_date_time}');");
        if let Err(err) = connection.execute(query) {
            error!("Failed to log request: {:?}", err);
            false
        } else {
            true
        }
    }

    pub async fn get_stats(&self) -> Vec<DatabaseStats> {
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return Vec::new();
            }
        };

        let query = "SELECT r.url, r.redirect, COUNT(l.id) AS log_count, r.comment
                            FROM redirects r
                            LEFT JOIN log l ON r.redirect = l.redirect
                            GROUP BY r.url, r.redirect;";
        let mut statement = match connection.prepare(query) {
            Ok(stmt) => stmt,
            Err(err) => {
                error!("Failed to prepare query: {:?}", err);
                return Vec::new();
            }
        };

        let mut res = Vec::new();
        while let Ok(State::Row) = statement.next() {
            if let Ok(url) = statement.read::<String, _>(0) {
                if let Ok(code) = statement.read::<String, _>(1) {
                    if let Ok(clicks) = statement.read::<i64, _>(2) {
                        if let Ok(comment) = statement.read::<Option<String>, _>(3) {
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
        res
    }

    pub async fn get_logs(&self, code: String) -> Vec<DatabaseLog> {
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return Vec::new();
            }
        };

        let mut res = Vec::new();
        let mut statement = match connection.prepare("SELECT * FROM log WHERE redirect = ?") {
            Ok(stmt) => stmt,
            Err(err) => {
                error!("Failed to prepare query: {:?}", err);
                return Vec::new();
            }
        };

        if let Err(err) = statement.bind((1, code.as_str())) {
            error!("Failed to bind parameter: {:?}", err);
            return Vec::new();
        }

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
        res
    }

    pub async fn insert_token(&self, token: String) -> bool {
        if check_string_injection(&token) {
            warn!("Request failed injection test: {token}");
            return false;
        }
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return false;
            }
        };

        let expires = chrono::offset::Local::now()
            .checked_add_signed(chrono::Duration::hours(1))
            .unwrap();

        let query = "INSERT INTO tokens (token, expiration) VALUES (?, ?);";
        let mut statement = match connection.prepare(query) {
            Ok(stmt) => stmt,
            Err(err) => {
                error!("Failed to prepare query: {:?}", err);
                return false;
            }
        };

        if let Err(err) = statement.bind(&[(1, token.as_str()), (2, &expires.to_rfc3339())][..]) {
            error!("Failed to bind parameters: {:?}", err);
            return false;
        }

        if let Err(err) = statement.next() {
            error!("Failed to insert token: {:?}", err);
            false
        } else {
            true
        }
    }

    pub async fn check_token(&self, token: String) -> bool {
        if check_string_injection(&token) {
            warn!("Request failed injection test: {token}");
            return false;
        }
        let connection = match sqlite::open(&self.filename) {
            Ok(conn) => conn,
            Err(err) => {
                error!("Failed to open database: {:?}", err);
                return false;
            }
        };

        println!("Checking token {token}");
        let query = "SELECT * FROM tokens WHERE token = ?;";
        let mut statement = match connection.prepare(query) {
            Ok(stmt) => stmt,
            Err(err) => {
                error!("Failed to prepare query: {:?}", err);
                return false;
            }
        };

        if let Err(err) = statement.bind((1, token.as_str())) {
            error!("Failed to bind parameter: {:?}", err);
            return false;
        }

        if let Ok(State::Row) = statement.next() {
            if let Ok(expires) = statement.read::<String, _>(2) {
                if let Ok(expires) = chrono::DateTime::parse_from_rfc3339(expires.as_str()) {
                    if expires > chrono::offset::Local::now() {
                        let query = "DELETE FROM tokens WHERE expiration < ?;";
                        let mut statement = match connection.prepare(query) {
                            Ok(stmt) => stmt,
                            Err(err) => {
                                error!("Failed to prepare query: {:?}", err);
                                return false;
                            }
                        };

                        if let Err(err) =
                            statement.bind((1, chrono::offset::Local::now().to_rfc3339().as_str()))
                        {
                            error!("Failed to bind parameter: {:?}", err);
                            return false;
                        }

                        if let Err(err) = statement.next() {
                            error!("Failed to delete expired tokens: {:?}", err);
                        }
                        return true;
                    } else {
                        info!("Expired token was used");
                        let query = "DELETE FROM tokens WHERE token = ?;";
                        let mut statement = match connection.prepare(query) {
                            Ok(stmt) => stmt,
                            Err(err) => {
                                error!("Failed to prepare query: {:?}", err);
                                return false;
                            }
                        };

                        if let Err(err) = statement.bind((1, token.as_str())) {
                            error!("Failed to bind parameter: {:?}", err);
                            return false;
                        }

                        if let Err(err) = statement.next() {
                            error!("Failed to delete token: {:?}", err);
                        }
                        return false;
                    }
                } else {
                    error!("Timestamp was unparse-able for token {token}");
                }
            } else {
                error!("Failed to read expiration from token {token}");
            }
        }
        false
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

    #[tokio::test]
    async fn login() {
        dotenv::dotenv().ok();
        env_logger::init();
        let db = Database::new();
        // assert!(db.insert_token("asdf".to_string()).await);
        assert!(db.check_token("asdf".to_string()).await);
    }
}
