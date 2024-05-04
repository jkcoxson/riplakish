CREATE TABLE log (id INTEGER PRIMARY KEY AUTOINCREMENT, timestamp DATETIME, redirect TEXT, url TEXT, ip TEXT);
CREATE TABLE redirects (id INTEGER PRIMARY KEY AUTOINCREMENT, url TEXT, redirect TEXT, comment TEXT);
CREATE TABLE tokens (id INTEGER PRIMARY KEY AUTOINCREMENT, token TEXT, expiration DATETIME);