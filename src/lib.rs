#![cfg(target_arch = "wasm32")]

// Cloudflare port of Riplakish

use serde::{Deserialize, Serialize};
use worker::*;

#[derive(Deserialize)]
struct Stat {
    url: String,
    redirect: String,
    log_count: u32,
    comment: Option<String>,
}

#[derive(Serialize)]
struct SerStat {
    url: String,
    code: String,
    comment: String,
    visits: u32,
}

#[derive(Serialize, Deserialize)]
struct Log {
    timestamp: String,
    ip: String,
    url: String,
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    // Create an instance of the Router, which can use parameters (/user/:name) or wildcard values
    // (/file/*pathname). Alternatively, use `Router::with_data(D)` and pass in arbitrary data for
    // routes to access and share using the `ctx.data()` method.
    let router = Router::new();

    // useful for JSON APIs
    #[derive(Deserialize, Serialize)]
    struct Account {
        id: u64,
        // ...
    }
    router
        .get_async("/admin", |_req, _ctx| async move {
            Response::from_html(include_str!("../frontend/dist/index.html"))
        })
        .get_async("/scripts.js", |_req, _ctx| async move {
            let mut headers = Headers::new();
            headers.append("Content-Type", "text/javascript")?;
            Ok(Response::ok(include_str!("../frontend/dist/scripts.js"))
                .unwrap()
                .with_headers(headers))
        })
        .get_async("/styles.css", |_req, _ctx| async move {
            let mut headers = Headers::new();
            headers.append("Content-Type", "text/css")?;
            Ok(Response::ok(include_str!("../frontend/dist/styles.css"))
                .unwrap()
                .with_headers(headers))
        })
        // REMOVE BEFORE DEPLOY
        .get_async("/headers", |req, _ctx| async move {
            // return the headers
            let headers = req.headers();
            Response::ok(format!("{:?}", headers))
        })
        .get_async("/r/:code", |req, ctx| async move {
            let d1 = ctx.env.d1("riplakish")?;
            let code = match ctx.param("code") {
                Some(c) => c,
                None => {
                    return Response::error("Bad Request", 400);
                }
            };

            // Check for injection
            if check_string_injection(code) {
                return Response::error("Bad Request", 400);
            }

            let statement = d1.prepare("SELECT url FROM redirects WHERE redirect = ?");
            let query = statement.bind(&[code.into()])?;
            let result = query.first::<String>(Some("url")).await?;
            let res = match result {
                Some(r) => r,
                None => return Response::error("Not found", 404),
            };

            let url = match Url::parse(&res) {
                Ok(u) => u,
                Err(_) => return Response::error("Bad URL", 500),
            };

            // Log the redirect
            let ip = req
                .headers()
                .get("CF-Connecting-IP")?
                .unwrap_or("unknown".to_string()); // I don't think this works in dev???
            let statement =
                d1.prepare("INSERT INTO log (redirect, ip, url, timestamp) VALUES (?, ?, ?, ?)");
            let query = statement.bind(&[
                code.into(),
                ip.into(),
                url.to_string().into(),
                chrono::offset::Local::now()
                    .format("%m/%d/%Y %T")
                    .to_string()
                    .into(),
            ])?;
            if let Err(e) = query.run().await {
                return Response::error(e.to_string(), 500);
            }

            Response::redirect(url)
        })
        .get_async("/admin/login", |req, ctx| async move {
            let d1 = ctx.env.d1("riplakish")?;
            let username = ctx.env.var("USERNAME")?.to_string();
            let password = ctx.env.var("PASSWORD")?.to_string();
            wasm_rs_dbg::dbg!(&username);
            wasm_rs_dbg::dbg!(&password);

            let input_username = req.headers().get("X-Username")?;
            let input_password = req.headers().get("X-Password")?;
            wasm_rs_dbg::dbg!(&input_username);
            wasm_rs_dbg::dbg!(&input_password);

            if input_username == Some(username) && input_password == Some(password) {
                // Generate a token
                let mut buf = [0; 16];
                let _ = getrandom::getrandom(&mut buf);
                let mut token = String::new();
                for c in buf {
                    token.push_str(&format!("{:02X}", c));
                }

                // Set the token
                // INSERT INTO tokens (token, expiration) VALUES (?, ?);
                let expires = chrono::offset::Local::now()
                    .checked_add_signed(chrono::Duration::hours(1))
                    .unwrap();
                let statement = d1.prepare("INSERT INTO tokens (token, expiration) VALUES (?, ?)");
                let query = statement.bind(&[token.clone().into(), expires.to_rfc3339().into()])?;

                if let Err(e) = query.run().await {
                    return Response::error(e.to_string(), 500);
                }

                // Set the X-Token header
                let mut headers = Headers::new();
                headers.append(
                    "Set-Cookie",
                    format!("X-Token={token}; SameSite=Strict; HttpOnly").as_str(),
                )?;
                Ok(Response::ok("")?.with_headers(headers))
            } else {
                Response::error("Unauthorized", 401)
            }
        })
        .get_async("/base", |_, ctx| async move {
            Response::ok(ctx.env.var("BASE_URL")?.to_string())
        })
        .get_async("/admin/stats", |req, ctx| async move {
            let d1 = ctx.env.d1("riplakish")?;

            if !check_token(&req.headers(), &d1).await {
                return Response::error("Unauthorized", 401);
            }

            let query = "SELECT r.url, r.redirect, COUNT(l.id) AS log_count, r.comment
            FROM redirects r
            LEFT JOIN log l ON r.redirect = l.redirect
            GROUP BY r.url, r.redirect;";

            let statement = d1.prepare(query);
            let result = statement.all().await?;
            match result.results::<Stat>() {
                Ok(r) => Response::from_json(
                    &r.iter()
                        .map(|r| SerStat {
                            url: r.url.clone(),
                            code: r.redirect.clone(),
                            comment: r.comment.clone().unwrap_or_default(),
                            visits: r.log_count,
                        })
                        .collect::<Vec<SerStat>>(),
                ),
                Err(_) => Response::error("Failed to query", 500),
            }
        })
        .get_async("/admin/logs/:code", |req, ctx| async move {
            let d1 = ctx.env.d1("riplakish")?;

            if !check_token(&req.headers(), &d1).await {
                return Response::error("Unauthorized", 401);
            }

            let code = match ctx.param("code") {
                Some(c) => c,
                None => {
                    return Response::error("Bad Request", 400);
                }
            };

            // Check for injection
            if check_string_injection(code) {
                return Response::error("Bad Request", 400);
            }

            // SELECT * FROM log WHERE redirect = ?
            let statement = d1.prepare("SELECT * FROM log WHERE redirect = ?");
            let query = statement.bind(&[code.into()])?;
            let result = query.all().await?;
            match result.results::<Log>() {
                Ok(r) => Response::from_json(&r),
                Err(_) => Response::error("Failed to query", 500),
            }
        })
        .post_async("/admin/add/*url", |req, ctx| async move {
            let d1 = ctx.env.d1("riplakish")?;

            if !check_token(&req.headers(), &d1).await {
                return Response::error("Unauthorized", 401);
            }

            let url = match ctx.param("url") {
                Some(u) => u,
                None => {
                    return Response::error("Bad Request", 400);
                }
            };

            // Check for injection
            if check_string_injection(url) {
                return Response::error("Bad Request", 400);
            }

            let mut buf = [0; 2];
            let _ = getrandom::getrandom(&mut buf);
            let mut code = String::new();
            for c in buf {
                code.push_str(&format!("{:02X}", c));
            }

            let statement = d1.prepare("INSERT INTO redirects (url, redirect) VALUES (?, ?)");

            let query = statement.bind(&[url.into(), code.into()])?;
            if let Err(e) = query.run().await {
                return Response::error(e.to_string(), 500);
            }

            Response::ok("Success")
        })
        .delete_async("/admin/remove/:code", |req, ctx| async move {
            let d1 = ctx.env.d1("riplakish")?;

            if !check_token(&req.headers(), &d1).await {
                return Response::error("Unauthorized", 401);
            }

            let code = match ctx.param("code") {
                Some(c) => c,
                None => {
                    return Response::error("Bad Request", 400);
                }
            };

            // Check for injection
            if check_string_injection(code) {
                return Response::error("Bad Request", 400);
            }

            // DELETE FROM redirects WHERE redirect = '{code}';
            let statement = d1.prepare("DELETE FROM redirects WHERE redirect = ?");
            let query = statement.bind(&[code.into()])?;
            if let Err(e) = query.run().await {
                return Response::error(e.to_string(), 500);
            }

            Response::ok("Success")
        })
        .post_async("/admin/modify/:code/*new_url", |req, ctx| async move {
            let d1 = ctx.env.d1("riplakish")?;

            if !check_token(&req.headers(), &d1).await {
                return Response::error("Unauthorized", 401);
            }

            let code = match ctx.param("code") {
                Some(c) => c,
                None => {
                    return Response::error("Bad Request", 400);
                }
            };

            let new_url = match ctx.param("new_url") {
                Some(u) => u,
                None => {
                    return Response::error("Bad Request", 400);
                }
            };

            // Check for injection
            if check_string_injection(code) || check_string_injection(new_url) {
                return Response::error("Bad Request", 400);
            }

            // UPDATE redirects SET url = '{url}' WHERE redirect = '{code}';
            let statement = d1.prepare("UPDATE redirects SET url = ? WHERE redirect = ?");
            let query = statement.bind(&[new_url.into(), code.into()])?;
            if let Err(e) = query.run().await {
                return Response::error(e.to_string(), 500);
            }

            Response::ok("Success")
        })
        .post_async(
            "/admin/modify-comment/:code/*new_comment",
            |req, ctx| async move {
                let d1 = ctx.env.d1("riplakish")?;

                if !check_token(&req.headers(), &d1).await {
                    return Response::error("Unauthorized", 401);
                }

                let code = match ctx.param("code") {
                    Some(c) => c,
                    None => {
                        return Response::error("Bad Request", 400);
                    }
                };

                let new_comment = match ctx.param("new_comment") {
                    Some(c) => c,
                    None => {
                        return Response::error("Bad Request", 400);
                    }
                };

                // Check for injection
                if check_string_injection(code) || check_string_injection(&new_comment) {
                    return Response::error("Bad Request", 400);
                }

                let new_comment = new_comment.replace("%20", " ");

                // UPDATE redirects SET comment = ? WHERE redirect = ?;
                let statement = d1.prepare("UPDATE redirects SET comment = ? WHERE redirect = ?");
                let query = statement.bind(&[new_comment.into(), code.into()])?;
                if let Err(e) = query.run().await {
                    return Response::error(e.to_string(), 500);
                }

                Response::ok("Success")
            },
        )
        .run(req, env)
        .await
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

#[inline]
async fn get_token(headers: &Headers) -> Option<String> {
    let cookies = headers.get("cookie").ok()??;
    let cookies = cookies.split(';').collect::<Vec<&str>>();
    for cookie in cookies {
        if cookie.starts_with("X-Token=") {
            let token = cookie.split('=').collect::<Vec<&str>>()[1];
            let token = token.replace(" SameSite", "");

            return Some(token);
        }
    }
    None
}

async fn check_token(headers: &Headers, d1: &D1Database) -> bool {
    match get_token(headers).await {
        Some(token) => {
            if check_string_injection(&token) {
                return false;
            }

            // check the token
            let statement = d1.prepare("SELECT expiration FROM tokens WHERE token = ?");
            let query = match statement.bind(&[token.into()]) {
                Ok(q) => q,
                Err(_) => return false,
            };
            let result = match query.first::<String>(Some("expiration")).await {
                Ok(r) => r,
                Err(_) => return false,
            };

            let res = match result {
                Some(r) => r,
                None => return false,
            };

            let expires = match chrono::DateTime::parse_from_rfc3339(&res) {
                Ok(e) => e,
                Err(_) => return false,
            };

            if chrono::offset::Local::now() > expires {
                return false;
            }

            // Delete old tokens
            let statement = d1.prepare("DELETE FROM tokens WHERE expiration < ?");
            if let Ok(query) = statement.bind(&[chrono::offset::Local::now().to_rfc3339().into()]) {
                let _ = query.run().await;
            }

            true
        }
        None => false,
    }
}
