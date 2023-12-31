use actix::prelude::*;
use ethers::core::{
    abi::Address,
    types::{
        transaction::{eip2718::TypedTransaction, eip712::TypedData},
        H256,
    },
};
use ethers_signers_browser_frontend::ws::messages::ChainInfo;
use log::{error, info, warn};
use rand::distributions::{Alphanumeric, DistString};
use std::{collections::HashMap, sync::mpsc};

/// Comm sends this message to sessions
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) enum WSRequest {
    Init { id: String, chain_id: u64, chains: Option<HashMap<u64, ChainInfo>> },
    Accounts { id: String },
    SignBinaryMessage { id: String, address: Address, message: H256 },
    SignTextMessage { id: String, address: Address, message: String },
    SignTransaction { id: String, transaction: TypedTransaction },
    SignTypedData { id: String, address: Address, typed_data: TypedData },
    Close { reason: String },
}

type WebsocketClient = Recipient<WSRequest>;

/// Sessions send this message to comm
#[derive(Clone, Message)]
#[rtype(result = "()")]
pub(super) enum WSReply {
    Connect { client: WebsocketClient },
    Init { id: String, client: WebsocketClient },
    Accounts { id: String, client: WebsocketClient, accounts: Vec<Address> },
    MessageSignature { id: String, client: WebsocketClient, signature: String },
    TransactionSignature { id: String, client: WebsocketClient, signature: String },
    Error { id: String, client: WebsocketClient, error: String },
    Disconnect { client: WebsocketClient },
}

/// Server sends this message to comm
#[derive(Clone, Message, Debug)]
#[rtype(result = "()")]
pub(super) struct AsyncRequest {
    pub id: String,
    pub content: AsyncRequestContent,
}

#[derive(Clone, Debug)]
pub(super) enum AsyncRequestContent {
    Accounts {},
    SignTextMessage { address: Address, message: String },
    SignBinaryMessage { address: Address, message: H256 },
    SignTransaction { transaction: TypedTransaction },
    SignTypedData { address: Address, typed_data: TypedData },
}

/// Comm sends this message to the server
#[derive(Clone, Debug)]
pub(super) struct AsyncResponse {
    pub id: String,
    pub content: AsyncResponseContent,
}

#[derive(Clone, Debug)]
pub(super) enum AsyncResponseContent {
    Accounts { accounts: Vec<Address> },
    MessageSignature { signature: String },
    TransactionSignature { signature: String },
    Error { error: String },
}

/// `CommServer` manages clients and forward server requests to them.
#[derive(Debug)]
pub(super) struct CommServer {
    server: mpsc::Sender<AsyncResponse>,
    chain_id: u64,
    chains: Option<HashMap<u64, ChainInfo>>,
    client: Option<WebsocketClient>,
    init_status: InitStatus,
    is_handling_request: bool,
    pending_messages: Vec<AsyncRequest>,
}

#[derive(Debug, PartialEq, Clone)]
enum InitStatus {
    None,
    Pending { id: String },
    Done,
}

impl CommServer {
    pub fn new(
        server: mpsc::Sender<AsyncResponse>,
        chain_id: u64,
        chains: Option<HashMap<u64, ChainInfo>>,
    ) -> CommServer {
        CommServer {
            client: None,
            server,
            chain_id,
            chains,
            init_status: InitStatus::None,
            is_handling_request: false,
            pending_messages: vec![],
        }
    }

    fn gen_id(&self) -> String {
        Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
    }
}

impl CommServer {
    fn is_same_client(&self, addr: &WebsocketClient) -> bool {
        matches!(self.client, Some(ref client) if client == addr)
    }

    fn is_client_init(&self) -> bool {
        self.init_status == InitStatus::Done
    }

    fn has_ready_client(&self) -> bool {
        self.client.is_some() && self.is_client_init()
    }

    fn kick_client(&self, client: &Recipient<WSRequest>, reason: &str) {
        warn!("kicking client: {}", reason);
        client.do_send(WSRequest::Close { reason: reason.to_string() });
    }

    fn kick_current_client(&mut self, reason: &str) {
        if let Some(ref addr) = self.client {
            self.kick_client(addr, reason);
            self.cleanup_client();
        }
    }

    fn cleanup_client(&mut self) {
        self.client = None;
        self.init_status = InitStatus::None;
        self.is_handling_request = false;
    }
}

impl CommServer {
    fn send_pending_message(&mut self) {
        if self.is_handling_request || !self.has_ready_client() {
            return
        }
        if let Some(msg) = self.pending_messages.first() {
            self.is_handling_request = true;
            self.client.as_ref().unwrap().do_send({
                let AsyncRequest { id, content } = msg.clone();
                match content {
                    AsyncRequestContent::Accounts {} => WSRequest::Accounts { id },
                    AsyncRequestContent::SignTextMessage { address, message } => {
                        WSRequest::SignTextMessage { id, address, message }
                    }
                    AsyncRequestContent::SignBinaryMessage { address, message } => {
                        WSRequest::SignBinaryMessage { id, address, message }
                    }
                    AsyncRequestContent::SignTransaction { transaction } => {
                        WSRequest::SignTransaction { id, transaction }
                    }
                    AsyncRequestContent::SignTypedData { address, typed_data } => {
                        WSRequest::SignTypedData { id, address, typed_data }
                    }
                }
            });
        }
    }

    fn handle_init(&mut self, id: String) {
        match self.init_status.clone() {
            InitStatus::Pending { id: original_id } => {
                if original_id != id {
                    self.kick_current_client("invalid id on init");
                    return
                }
                self.init_status = InitStatus::Done;
                self.send_pending_message();
            }
            _ => self.kick_current_client("init already done"),
        }
    }

    fn send_server_reply(&mut self, reply: AsyncResponse) {
        match self.server.send(reply) {
            Ok(_) => {}
            Err(e) => {
                error!("failed to send response to server: {:?}", e);
            }
        }
    }

    fn handle_response(&mut self, id: String, content: AsyncResponseContent) {
        if !self.is_client_init() {
            match self.init_status.clone() {
                InitStatus::Pending { id: original_id } => {
                    if original_id != id {
                        self.kick_current_client("invalid id on init");
                        return
                    }
                    if let AsyncResponseContent::Error { .. } = content {
                        if let Some(msg) = self.pending_messages.first() {
                            // Basically we cheat a little bit to be able to send the error message
                            // to the server despite init being sort of
                            // implicit
                            self.send_server_reply(AsyncResponse {
                                id: msg.id.clone(),
                                content: content.clone(),
                            });
                        }
                    }
                    self.kick_current_client(format!("failed init: {:?}", content).as_str());
                }
                _ => {
                    self.kick_current_client("wrong init status");
                }
            }
            return
        }

        if let Some(msg) = self.pending_messages.first() {
            if msg.id != id {
                print!("invalid response id ({} vs {}), ignore and send the next one", msg.id, id);
            } else {
                self.pending_messages.remove(0);
                self.send_server_reply(AsyncResponse { id, content });
            }
        } else {
            print!("no pending message, ignore and send the next one");
        }

        self.is_handling_request = false;
        self.send_pending_message();
    }

    fn queue_pending_message(&mut self, msg: AsyncRequest) {
        self.pending_messages.push(msg);
        self.send_pending_message();
    }
}

impl Actor for CommServer {
    type Context = Context<Self>;
}

// from client
impl Handler<WSReply> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: WSReply, _: &mut Context<Self>) -> Self::Result {
        match msg {
            WSReply::Connect { client } => {
                info!("Browser connected");
                self.client = Some(client.clone());
                let id = self.gen_id();
                self.init_status = InitStatus::Pending { id: id.clone() };
                client.do_send(WSRequest::Init {
                    id,
                    chain_id: self.chain_id,
                    chains: self.chains.clone(),
                });
            }
            WSReply::Disconnect { client } => {
                info!("Browser disconnected");
                if !self.is_same_client(&client) {
                    return
                }
                self.cleanup_client();
            }
            WSReply::Init { id, client } => {
                if !self.is_same_client(&client) {
                    self.kick_client(&client, "invalid client");
                    return
                }
                self.handle_init(id);
            }
            WSReply::Accounts { id, client, accounts } => {
                if !self.is_same_client(&client) {
                    self.kick_client(&client, "invalid client");
                    return
                }
                self.handle_response(id, AsyncResponseContent::Accounts { accounts });
            }
            WSReply::MessageSignature { id, client, signature } => {
                if !self.is_same_client(&client) {
                    self.kick_client(&client, "invalid client");
                    return
                }
                self.handle_response(id, AsyncResponseContent::MessageSignature { signature });
            }
            WSReply::TransactionSignature { id, client, signature } => {
                if !self.is_same_client(&client) {
                    self.kick_client(&client, "invalid client");
                    return
                }
                self.handle_response(id, AsyncResponseContent::TransactionSignature { signature });
            }
            WSReply::Error { id, client, error } => {
                if !self.is_same_client(&client) {
                    self.kick_client(&client, "invalid client");
                    return
                }
                self.handle_response(id, AsyncResponseContent::Error { error });
            }
        }
    }
}

// from server
impl Handler<AsyncRequest> for CommServer {
    type Result = ();

    fn handle(&mut self, msg: AsyncRequest, _: &mut Context<Self>) {
        self.queue_pending_message(msg);
    }
}
