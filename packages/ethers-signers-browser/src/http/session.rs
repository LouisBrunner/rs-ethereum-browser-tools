use super::comm;
use actix::{prelude::*, Actor, StreamHandler};
use actix_web_actors::ws;
use bytestring::ByteString;
use ethers_signers_browser_frontend::ws::messages::{
    Request, RequestContent, Response, ResponseContent,
};
use log::{error, warn};
use serde_json::Result as SerdeResult;
use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct WSFlow {
    comm: Addr<comm::CommServer>,
    last_heartbeat: Instant,
}

impl WSFlow {
    pub fn new(comm: Addr<comm::CommServer>) -> Self {
        Self { comm, last_heartbeat: Instant::now() }
    }

    fn forward_to_client(&self, msg: comm::WSRequest) -> Result<SerdeResult<String>, String> {
        let msg = match msg {
            comm::WSRequest::Init { id, chain_id } => {
                Request { id, content: RequestContent::Init { chain_id } }
            }
            comm::WSRequest::Accounts { id } => {
                Request { id, content: RequestContent::Accounts {} }
            }
            comm::WSRequest::SignTextMessage { id, address, message } => {
                Request { id, content: RequestContent::SignTextMessage { address, message } }
            }
            comm::WSRequest::SignBinaryMessage { id, address, message } => {
                Request { id, content: RequestContent::SignBinaryMessage { address, message } }
            }
            comm::WSRequest::SignTransaction { id, transaction } => {
                Request { id, content: RequestContent::SignTransaction { transaction } }
            }
            comm::WSRequest::SignTypedData { id, address, typed_data } => {
                Request { id, content: RequestContent::SignTypedData { address, typed_data } }
            }
            comm::WSRequest::Close { reason } => {
                return Err(reason);
            }
        };
        Ok(serde_json::to_string(&msg))
    }

    fn forward_to_server(
        &self,
        ctx: &mut <Self as Actor>::Context,
        text: ByteString,
    ) -> SerdeResult<()> {
        let addr = ctx.address().recipient();
        let response: Response = serde_json::from_str(&text)?;
        match response.content {
            ResponseContent::Init {} => {
                self.comm.do_send(comm::WSReply::Init { id: response.id, client: addr });
            }
            ResponseContent::Accounts { addresses } => {
                self.comm.do_send(comm::WSReply::Accounts {
                    id: response.id,
                    client: addr,
                    accounts: addresses,
                });
            }
            ResponseContent::Signature { signature } => {
                self.comm.do_send(comm::WSReply::Signature {
                    id: response.id,
                    client: addr,
                    signature,
                });
            }
            ResponseContent::Error { error } => {
                self.comm.do_send(comm::WSReply::Error { id: response.id, client: addr, error });
            }
        };
        Ok(())
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });
    }

    fn close(
        &self,
        ctx: &mut <Self as Actor>::Context,
        reason: String,
        user_reason: Option<String>,
    ) {
        error!("closing websocket: {}", reason);
        ctx.close(Some(ws::CloseReason {
            code: ws::CloseCode::Error,
            description: Some(user_reason.unwrap_or(reason)),
        }));
        ctx.stop();
    }
}

impl Actor for WSFlow {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);

        let addr = ctx.address().recipient();
        self.comm.do_send(comm::WSReply::Connect { client: addr });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        let addr = ctx.address().recipient();
        self.comm.do_send(comm::WSReply::Disconnect { client: addr });
        Running::Stop
    }
}

// Comm -> Websocket
impl Handler<comm::WSRequest> for WSFlow {
    type Result = ();

    fn handle(&mut self, msg: comm::WSRequest, ctx: &mut Self::Context) {
        match self.forward_to_client(msg) {
            Ok(text) => match text {
                Ok(text) => {
                    ctx.text(text);
                }
                Err(e) => {
                    self.close(
                        ctx,
                        format!("error forwarding message: {}", e),
                        Some("internal error (client)".to_owned()),
                    );
                }
            },
            Err(msg) => {
                self.close(ctx, msg, None);
            }
        }
    }
}

// Front-end -> Websocket
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WSFlow {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.last_heartbeat = Instant::now();
                ctx.pong(&msg)
            }
            Ok(ws::Message::Pong(_msg)) => {
                self.last_heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                match self.forward_to_server(ctx, text) {
                    Ok(_) => (),
                    Err(e) => {
                        self.close(
                            ctx,
                            format!("error forwarding message: {}", e),
                            Some("internal error (server)".to_owned()),
                        );
                    }
                };
            }
            Ok(ws::Message::Close(reason)) => {
                warn!("WS Closed: {:?}", reason);
                ctx.stop();
            }
            Err(e) => {
                error!("WS Error: {}", e);
            }
            _ => {
                self.close(ctx, "unsupported message received".to_owned(), None);
            }
        }
    }
}
