use super::comm;
use actix::{prelude::*, Actor, StreamHandler};
use actix_web_actors::ws;
use bytestring::ByteString;
use ethers_signers_browser_frontend::ws::messages::{
    Request, RequestContent, Response, ResponseContent,
};
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

    fn forward_to_client(&self, msg: comm::WSMessage) -> SerdeResult<String> {
        let msg = match msg {
            comm::WSMessage::Init { chain_id } => {
                Request { id: "unused".to_owned(), content: RequestContent::Init { chain_id } }
            }
            comm::WSMessage::SignMessage { message } => Request {
                id: "unused".to_owned(),
                content: RequestContent::SignMessage { message },
            },
            comm::WSMessage::SignTransaction { transaction } => Request {
                id: "unused".to_owned(),
                content: RequestContent::SignTransaction { transaction },
            },
            comm::WSMessage::SignTypedData { typed_data } => Request {
                id: "unused".to_owned(),
                content: RequestContent::SignTypedData { typed_data },
            },
        };
        serde_json::to_string(&msg)
    }

    fn forward_to_server(
        &self,
        ctx: &mut <Self as Actor>::Context,
        text: ByteString,
    ) -> SerdeResult<()> {
        let addr = ctx.address().recipient();
        let response: Response = serde_json::from_str(&text)?;
        match response.content {
            ResponseContent::Init { addresses } => {
                self.comm.do_send(comm::InitReply { addr, accounts: addresses });
            }
            ResponseContent::Signature { signature } => {
                self.comm.do_send(comm::SignatureReply { addr, signature });
            }
            ResponseContent::Error { error } => {
                self.comm.do_send(comm::ErrorReply { addr, error });
            }
        };
        Ok(())
    }

    fn heartbeat(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            if Instant::now().duration_since(act.last_heartbeat) > CLIENT_TIMEOUT {
                ctx.stop();
                return
            }

            ctx.ping(b"");
        });
    }
}

impl Actor for WSFlow {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.heartbeat(ctx);

        let addr = ctx.address().recipient();
        self.comm.do_send(comm::Connect { addr });
    }

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        let addr = ctx.address().recipient();
        self.comm.do_send(comm::Disconnect { addr });
        Running::Stop
    }
}

// Comm -> Websocket
impl Handler<comm::WSMessage> for WSFlow {
    type Result = ();

    fn handle(&mut self, msg: comm::WSMessage, ctx: &mut Self::Context) {
        match self.forward_to_client(msg) {
            Ok(text) => ctx.text(text),
            Err(e) => {
                println!("Error forwarding message: {}", e);
                ctx.stop();
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
                        println!("Error forwarding message: {}", e);
                        ctx.stop();
                    }
                };
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(e) => println!("WS Error: {}", e),
            _ => ctx.stop(),
        }
    }
}
