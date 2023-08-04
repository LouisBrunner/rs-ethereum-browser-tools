use actix::{Actor, Addr};
use actix_web::{dev::ServerHandle, rt, web, App, HttpServer};
use ethers::core::{
    abi::Address,
    types::{
        transaction::{eip2718::TypedTransaction, eip712::TypedData},
        H256,
    },
};
use rand::distributions::{Alphanumeric, DistString};
use routes::{dist, index, ws_open};
use std::{
    sync::{mpsc, Mutex},
    thread::{self, sleep},
    time::{Duration, Instant},
};

mod comm;
mod routes;
pub mod session;

// FIXME: tweak those values
static TIMEOUT: Duration = Duration::MAX;

struct ServerData {
    port: u16,
    server: ServerHandle,
    comm: Addr<comm::CommServer>,
}

async fn run_app(
    nonce: String,
    comm: comm::CommServer,
    sender: mpsc::Sender<ServerData>,
    port: Option<u16>,
) -> std::io::Result<()> {
    let comm = comm.start();

    let comm_data = comm.clone();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(comm_data.clone()))
            .app_data(web::Data::new(nonce.clone()))
            .service(ws_open)
            .service(index)
            .service(dist)
    })
    .bind(("127.0.0.1", port.unwrap_or(0)))?;

    let addrs = server.addrs();
    let server = server.run();

    let _ = sender.send(ServerData {
        port: addrs[0].port(),
        server: server.handle(),
        comm: comm.clone(),
    });

    server.await
}

// add error
#[derive(thiserror::Error, Debug)]
pub enum ServerError {
    #[error("comm error: {0}")]
    Comm(String),
    #[error("client error: {0}")]
    Client(String),
}

pub struct ServerOptions {
    pub port: Option<u16>,
    pub nonce: Option<String>,
}

pub(super) struct Server {
    port: u16,
    nonce: String,
    server: ServerHandle,
    comm: Addr<comm::CommServer>,
    comm_receiver: Mutex<mpsc::Receiver<comm::AsyncResponse>>,
}

impl Server {
    pub async fn new(chain_id: u64, opts: Option<ServerOptions>) -> Result<Self, ServerError> {
        let (sender, receiver) = mpsc::channel();
        let (comm_sender, comm_receiver) = mpsc::channel();

        let opts = opts.unwrap_or(ServerOptions { port: None, nonce: None });
        let nonce = opts.nonce.unwrap_or(Alphanumeric.sample_string(&mut rand::thread_rng(), 16));

        {
            let nonce = nonce.clone();
            thread::spawn(move || {
                let server_future =
                    run_app(nonce, comm::CommServer::new(comm_sender, chain_id), sender, opts.port);
                rt::System::new().block_on(server_future)
            });
        }

        let data = receiver
            .recv()
            .map_err(|_: mpsc::RecvError| ServerError::Comm("internal error (init)".to_owned()))?;

        Ok(Self {
            port: data.port,
            server: data.server,
            nonce,
            comm: data.comm,
            comm_receiver: Mutex::new(comm_receiver),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn nonce(&self) -> String {
        self.nonce.clone()
    }

    pub async fn get_user_addresses(&self) -> Result<Vec<Address>, ServerError> {
        self.wait_for_reply(
            comm::AsyncRequestContent::Accounts {},
            |res| match res {
                comm::AsyncResponseContent::Accounts { accounts } => Some(accounts.clone()),
                _ => None,
            },
            TIMEOUT,
        )
        .await
    }

    pub async fn sign_text_message(
        &self,
        address: Address,
        message: String,
    ) -> Result<String, ServerError> {
        self.wait_for_reply(
            comm::AsyncRequestContent::SignTextMessage { address, message },
            |res| match res {
                comm::AsyncResponseContent::Signature { signature } => Some(signature.clone()),
                _ => None,
            },
            TIMEOUT,
        )
        .await
    }

    pub async fn sign_binary_message(
        &self,
        address: Address,
        message: H256,
    ) -> Result<String, ServerError> {
        self.wait_for_reply(
            comm::AsyncRequestContent::SignBinaryMessage { address, message },
            |res| match res {
                comm::AsyncResponseContent::Signature { signature } => Some(signature.clone()),
                _ => None,
            },
            TIMEOUT,
        )
        .await
    }

    pub async fn sign_transaction(
        &self,
        transaction: TypedTransaction,
    ) -> Result<String, ServerError> {
        self.wait_for_reply(
            comm::AsyncRequestContent::SignTransaction { transaction },
            |res| match res {
                comm::AsyncResponseContent::Signature { signature } => Some(signature.clone()),
                _ => None,
            },
            TIMEOUT,
        )
        .await
    }

    pub async fn sign_typed_data(
        &self,
        address: Address,
        typed_data: TypedData,
    ) -> Result<String, ServerError> {
        self.wait_for_reply(
            comm::AsyncRequestContent::SignTypedData { address, typed_data },
            |res| match res {
                comm::AsyncResponseContent::Signature { signature } => Some(signature.clone()),
                _ => None,
            },
            TIMEOUT,
        )
        .await
    }

    async fn stop(&self) -> Result<(), ServerError> {
        self.server.stop(false).await;
        Ok(())
    }

    async fn wait_for_reply<U>(
        &self,
        req_content: comm::AsyncRequestContent,
        pred: fn(&comm::AsyncResponseContent) -> Option<U>,
        timeout: Duration,
    ) -> Result<U, ServerError> {
        // TODO: should be wrapped in a mutex
        let id = self.gen_id();
        let req: comm::AsyncRequest = comm::AsyncRequest { id: id.clone(), content: req_content };
        self.comm
            .send(req)
            .await
            .map_err(|_| ServerError::Comm("internal error (comm)".to_owned()))?;

        // one request at a time
        let receiver = self.comm_receiver.lock().expect("poisoned mutex");

        let start = Instant::now();
        while start.elapsed() < timeout {
            let res = receiver.try_recv();
            match res {
                Ok(res) => {
                    if res.id == id {
                        return match pred(&res.content) {
                            Some(res) => Ok(res),
                            None => match res.content {
                                comm::AsyncResponseContent::Error { error } => {
                                    Err(ServerError::Client(error))
                                }
                                _ => Err(ServerError::Comm("unexpected response".to_string())),
                            },
                        };
                    }
                    // ignore ids that don't match
                }
                Err(mpsc::TryRecvError::Empty) => (),
                Err(mpsc::TryRecvError::Disconnected) => {
                    return Err(ServerError::Comm("internal error (disc)".to_string()))
                }
            }
            sleep(Duration::from_millis(100));
        }
        Err(ServerError::Comm("timeout".to_string()))
    }

    fn gen_id(&self) -> String {
        Alphanumeric.sample_string(&mut rand::thread_rng(), 16)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
