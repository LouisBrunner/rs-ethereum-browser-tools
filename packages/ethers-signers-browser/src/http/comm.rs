use actix::prelude::*;
use ethers_core::{
    abi::Address,
    types::{
        transaction::{eip2718::TypedTransaction, eip712::TypedData},
        H256,
    },
};
use std::sync::mpsc;

/// Server sends this messages to session
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) enum WSMessage {
    Init { chain_id: u64 },
    SignMessage { message: H256 },
    SignTransaction { transaction: TypedTransaction },
    SignTypedData { typed_data: TypedData },
}

type WebsocketClient = Recipient<WSMessage>;

/// Message for server communications

/// New session is created
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct Connect {
    pub addr: WebsocketClient,
}

/// Session is disconnected
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct Disconnect {
    pub addr: WebsocketClient,
}

/// Initialization request
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct InitRequest {
    pub chain_id: u64,
}

/// Initialization reply
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct InitReply {
    pub addr: WebsocketClient,
    pub accounts: Vec<Address>,
}

/// Sign message request
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct SignMessageRequest {
    pub message: H256,
}

/// Sign typed data request
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct SignTypedDataRequest {
    pub typed_data: TypedData,
}

/// Sign message request
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct SignTransactionRequest {
    pub transaction: TypedTransaction,
}

/// Sign message reply
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct SignatureReply {
    pub addr: WebsocketClient,
    pub signature: String,
}

/// Client error
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) struct ErrorReply {
    pub addr: WebsocketClient,
    pub error: String,
}

#[derive(thiserror::Error, Debug)]
pub(super) enum AsyncError {
    #[error("no client connected")]
    NoClient,
    #[error("{0}")]
    FromClient(String),
}

pub(super) enum AsyncResponse {
    ClientConnected,
    InitReply { accounts: Vec<Address> },
    SignatureReply { signature: String },
    Error(AsyncError),
}

/// `CommServer` manages clients and forward server requests to them.
#[derive(Debug)]
pub(super) struct CommServer {
    client: Option<WebsocketClient>,
    server: mpsc::Sender<AsyncResponse>,
}

impl CommServer {
    pub fn new(server: mpsc::Sender<AsyncResponse>) -> CommServer {
        CommServer { client: None, server }
    }
}

impl CommServer {}

impl Actor for CommServer {
    type Context = Context<Self>;
}

// from client
impl Handler<Connect> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Browser connected");
        self.client = Some(msg.addr);
        // FIXME: how to handle error?
        let _ = self.server.send(AsyncResponse::ClientConnected);
    }
}

impl Handler<Disconnect> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Browser disconnected");
        match self.client {
            Some(ref addr) if addr == &msg.addr => {
                self.client = None;
            }
            _ => (),
        }
    }
}

impl Handler<InitReply> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: InitReply, _: &mut Context<Self>) {
        match self.client {
            Some(ref addr) if addr == &msg.addr => {
                // FIXME: how to handle error?
                let _ = self.server.send(AsyncResponse::InitReply { accounts: msg.accounts });
            }
            _ => (),
        }
    }
}

impl Handler<SignatureReply> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: SignatureReply, _: &mut Context<Self>) {
        match self.client {
            Some(ref addr) if addr == &msg.addr => {
                // FIXME: how to handle error?
                let _ =
                    self.server.send(AsyncResponse::SignatureReply { signature: msg.signature });
            }
            _ => (),
        }
    }
}

impl Handler<ErrorReply> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: ErrorReply, _: &mut Context<Self>) {
        match self.client {
            Some(ref addr) if addr == &msg.addr => {
                let _ = self.server.send(AsyncResponse::Error(AsyncError::FromClient(msg.error)));
            }
            _ => (),
        }
    }
}

// from server
impl Handler<InitRequest> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: InitRequest, _: &mut Context<Self>) {
        match self.client {
            Some(ref addr) => {
                addr.do_send(WSMessage::Init { chain_id: msg.chain_id });
            }
            None => {
                // FIXME: how to handle error?
                let _ = self.server.send(AsyncResponse::Error(AsyncError::NoClient));
            }
        }
    }
}

impl Handler<SignMessageRequest> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: SignMessageRequest, _: &mut Context<Self>) {
        match self.client {
            Some(ref addr) => {
                addr.do_send(WSMessage::SignMessage { message: msg.message });
            }
            None => {
                // FIXME: how to handle error?
                let _ = self.server.send(AsyncResponse::Error(AsyncError::NoClient));
            }
        }
    }
}

impl Handler<SignTransactionRequest> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: SignTransactionRequest, _: &mut Context<Self>) {
        match self.client {
            Some(ref addr) => {
                addr.do_send(WSMessage::SignTransaction { transaction: msg.transaction });
            }
            None => {
                // FIXME: how to handle error?
                let _ = self.server.send(AsyncResponse::Error(AsyncError::NoClient));
            }
        }
    }
}

impl Handler<SignTypedDataRequest> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: SignTypedDataRequest, _: &mut Context<Self>) {
        match self.client {
            Some(ref addr) => {
                addr.do_send(WSMessage::SignTypedData { typed_data: msg.typed_data });
            }
            None => {
                // FIXME: how to handle error?
                let _ = self.server.send(AsyncResponse::Error(AsyncError::NoClient));
            }
        }
    }
}
