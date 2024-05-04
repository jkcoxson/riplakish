#![cfg(target_arch = "wasm32")]
use serde::{Deserialize, Serialize};
use worker::*;

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
        .get_async("/admin", |_req, ctx| async move {
            Response::from_html(include_str!("../frontend/dist/index.html"))
        })
        .get_async("/scripts.js", |_req, ctx| async move {
            let mut headers = Headers::new();
            headers.append("Content-Type", "text/javascript")?;
            Ok(Response::ok(include_str!("../frontend/dist/scripts.js"))
                .unwrap()
                .with_headers(headers))
        })
        .get_async("/styles.css", |_req, ctx| async move {
            let mut headers = Headers::new();
            headers.append("Content-Type", "text/css")?;
            Ok(Response::ok(include_str!("../frontend/dist/styles.css"))
                .unwrap()
                .with_headers(headers))
        })
        // handle files and fields from multipart/form-data requests
        .post_async("/upload", |mut req, _ctx| async move {
            let form = req.form_data().await?;
            if let Some(entry) = form.get("file") {
                match entry {
                    FormEntry::File(file) => {
                        let bytes = file.bytes().await?;
                    }
                    FormEntry::Field(_) => return Response::error("Bad Request", 400),
                }
                // ...

                if let Some(permissions) = form.get("permissions") {
                    // permissions == "a,b,c,d"
                }
                // or call `form.get_all("permissions")` if using multiple entries per field
            }

            Response::error("Bad Request", 400)
        })
        // read/write binary data
        .post_async("/echo-bytes", |mut req, _ctx| async move {
            let data = req.bytes().await?;
            if data.len() < 1024 {
                return Response::error("Bad Request", 400);
            }

            Response::from_bytes(data)
        })
        .run(req, env)
        .await
}
