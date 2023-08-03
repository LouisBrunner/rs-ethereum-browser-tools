use actix::{Actor, Addr, MailboxError};
use actix_web::{dev::ServerHandle, rt, web, App, HttpServer};
use ethers::core::{
    abi::Address,
    types::{
        transaction::{eip2718::TypedTransaction, eip712::TypedData},
        H256,
    },
};
use routes::{dist, index, ws_open};
use std::{
    sync::{mpsc, Mutex, PoisonError},
    thread::{self, sleep},
    time::{Duration, Instant},
};

mod comm;
mod routes;
pub mod session;

// FIXME: tweak those values
static TIMEOUT: Duration = Duration::MAX;
static CONNECT_TIMEOUT: Duration = Duration::MAX;

struct ServerData {
    port: u16,
    server: ServerHandle,
    comm: Addr<comm::CommServer>,
}

async fn run_app(comm: comm::CommServer, sender: mpsc::Sender<ServerData>) -> std::io::Result<()> {
    let comm = comm.start();

    let comm_data = comm.clone();
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(comm_data.clone()))
            .service(ws_open)
            .service(index)
            .service(dist)
    })
    .bind(("127.0.0.1", 0))?;

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
    #[error("recv error: {0}")]
    Recv(#[from] mpsc::RecvError),
    #[error("comm error: {0}")]
    Comm(String),
    #[error("client error: {0}")]
    Client(String),
}

impl From<MailboxError> for ServerError {
    fn from(err: MailboxError) -> Self {
        Self::Comm(err.to_string())
    }
}

impl<T> From<PoisonError<T>> for ServerError {
    fn from(err: PoisonError<T>) -> Self {
        Self::Comm(err.to_string())
    }
}

pub(super) struct Server {
    port: u16,
    server: ServerHandle,
    comm: Addr<comm::CommServer>,
    comm_receiver: Mutex<mpsc::Receiver<comm::AsyncResponse>>,
}

impl Server {
    pub async fn new() -> Result<Self, ServerError> {
        let (sender, receiver) = mpsc::channel();
        let (comm_sender, comm_receiver) = mpsc::channel();

        {
            thread::spawn(move || {
                let server_future = run_app(comm::CommServer::new(comm_sender), sender);
                rt::System::new().block_on(server_future)
            });
        }

        let data = receiver.recv()?;

        // TODO: might be nice to also generate a random nonce to pass as a query parameter
        Ok(Self {
            port: data.port,
            server: data.server,
            comm: data.comm,
            comm_receiver: Mutex::new(comm_receiver),
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub async fn get_user_addresses(&self, chain_id: u64) -> Result<Vec<Address>, ServerError> {
        self.wait_for_reply(
            |res| match res {
                Ok(comm::AsyncResponse::ClientConnected) => Some(()),
                _ => None,
            },
            CONNECT_TIMEOUT,
        )
        .await?;

        self.comm.send(comm::InitRequest { chain_id }).await?;
        self.wait_for_reply(
            |res| match res {
                Ok(comm::AsyncResponse::InitReply { accounts }) => Some(accounts.clone()),
                _ => None,
            },
            TIMEOUT,
        )
        .await
    }

    pub async fn sign_message(&self, message: H256) -> Result<String, ServerError> {
        self.comm.send(comm::SignMessageRequest { message }).await?;
        self.wait_for_reply(
            |res| match res {
                Ok(comm::AsyncResponse::SignatureReply { signature }) => Some(signature.clone()),
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
        self.comm.send(comm::SignTransactionRequest { transaction }).await?;
        self.wait_for_reply(
            |res| match res {
                Ok(comm::AsyncResponse::SignatureReply { signature }) => Some(signature.clone()),
                _ => None,
            },
            TIMEOUT,
        )
        .await
    }

    #[allow(dead_code)] // FIXME: remove
    pub async fn sign_typed_data(&self, typed_data: TypedData) -> Result<String, ServerError> {
        self.comm.send(comm::SignTypedDataRequest { typed_data }).await?;
        self.wait_for_reply(
            |res| match res {
                Ok(comm::AsyncResponse::SignatureReply { signature }) => Some(signature.clone()),
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
        pred: fn(&Result<comm::AsyncResponse, mpsc::TryRecvError>) -> Option<U>,
        timeout: Duration,
    ) -> Result<U, ServerError> {
        // one request at a time
        let receiver = self.comm_receiver.lock()?;
        // TODO: id matching mechanism
        // TODO: would like the send to be in the loop
        let start = Instant::now();
        while start.elapsed() < timeout {
            let res = receiver.try_recv();
            if let Some(res) = pred(&res) {
                return Ok(res);
            }
            match res {
                Ok(comm::AsyncResponse::ClientConnected) => (),
                Ok(comm::AsyncResponse::Error(err)) => match err {
                    comm::AsyncError::NoClient => (),
                    comm::AsyncError::FromClient(err) => {
                        return Err(ServerError::Client(err.to_string()))
                    }
                },
                Ok(_) => return Err(ServerError::Comm("unexpected response".to_string())),
                Err(_) => (),
            }
            sleep(Duration::from_millis(100));
        }
        Err(ServerError::Comm("timeout".to_string()))
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
