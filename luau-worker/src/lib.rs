use futures_util::StreamExt;
use base64::prelude::*;
use luau_lifter::decompile_bytecode;
use serde::{Deserialize, Serialize};
use worker::{*, WebSocketPair, WebsocketEvent};

const AUTH_SECRET: &str = "JRxwJlG8AA8xiSmd6JWFWI56b4ForVMbEMHwrXTyF65rKy0ZvhuhCfifZSSOeqFZ";

#[derive(Deserialize)]
struct DecompileMessage {
    id: String,
    encoded_bytecode: String,
}

#[derive(Serialize)]
struct DecompileResponse {
    id: String,
    decompilation: String,
}

#[event(fetch, respond_with_errors)]
pub async fn main(req: Request, env: Env, _ctx: worker::Context) -> Result<Response> {
    console_error_panic_hook::set_once();

    let router = Router::new();

    router
        .get_async("/decompile_ws", |req, _ctx| async move {
            // Authorization header validation
            let license = req
                .headers()
                .get("Authorization")?
                .ok_or_else(|| worker::Error::RustError("authorization header is required".into()))?;

            if license != AUTH_SECRET {
                return Response::error("invalid license", 403);
            }

            // Initialize WebSocket pair
            let pair = WebSocketPair::new()?;
            let server = pair.server;
            server.accept()?;

            // Background task for WebSocket events
            wasm_bindgen_futures::spawn_local(async move {
                let mut events = server.events().expect("could not open stream");
                while let Some(event) = events.next().await {
                    if let Ok(WebsocketEvent::Message(msg)) = event {
                        if let Ok(data) = msg.json::<DecompileMessage>() {
                            if let Ok(bytecode) = BASE64_STANDARD.decode(data.encoded_bytecode) {
                                let resp = DecompileResponse {
                                    id: data.id,
                                    decompilation: decompile_bytecode(&bytecode, 1),
                                };
                                let _ = server.send_with_str(serde_json::to_string(&resp).unwrap());
                            } else {
                                let _ = server.send_with_str(
                                    "{\"error\":\"invalid base64 data\"}".to_string(),
                                );
                            }
                        } else {
                            let _ = server.send_with_str(
                                "{\"error\":\"invalid message format\"}".to_string(),
                            );
                        }
                    }
                }
            });

            Response::from_websocket(pair.client)
        })
        .post_async("/decompile", |mut req, _ctx| async move {
            // Authorization header validation
            let license = req
                .headers()
                .get("Authorization")?
                .ok_or_else(|| worker::Error::RustError("authorization header is required".into()))?;

            if license != AUTH_SECRET {
                return Response::error("invalid license", 403);
            }

            // Read and decode request body
            let body = req.bytes().await?;
            match BASE64_STANDARD.decode(body) {
                Ok(bytecode) => Response::ok(decompile_bytecode(&bytecode, 203)),
                Err(_) => Response::error("invalid bytecode", 400),
            }
        })
        .run(req, env)
        .await
}
