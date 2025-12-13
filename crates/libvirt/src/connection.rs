//! Connection management for libvirt RPC.
//!
//! This module handles:
//! - Serial number generation
//! - Request/response matching
//! - Concurrent request dispatch

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use bytes::Bytes;
use tokio::sync::{mpsc, oneshot, Mutex};

use crate::error::{Error, Result};
use crate::generated::{LibvirtRpc, RpcError};
use crate::packet::{Packet, Status};
use crate::transport::{Transport, UnixTransport};

/// Default Unix socket path for system connections.
pub const SYSTEM_SOCKET_PATH: &str = "/var/run/libvirt/libvirt-sock";

/// Default Unix socket path for session connections (relative to XDG_RUNTIME_DIR).
pub const SESSION_SOCKET_PATH: &str = "libvirt/libvirt-sock";

/// A connection to a libvirt daemon.
pub struct Connection {
    inner: Arc<ConnectionInner>,
}

struct ConnectionInner {
    /// Serial number counter.
    serial: AtomicU32,
    /// Sender to the writer task.
    tx: mpsc::Sender<WriteRequest>,
    /// Pending requests waiting for responses (keyed by serial as i32).
    pending: Mutex<HashMap<i32, oneshot::Sender<Result<Bytes>>>>,
}

struct WriteRequest {
    packet: Packet,
    response_tx: oneshot::Sender<Result<Bytes>>,
}

impl Connection {
    /// Connect to a libvirt daemon via Unix socket.
    pub async fn connect_unix(path: &str) -> Result<Self> {
        let transport = UnixTransport::connect(path).await?;
        Self::from_transport(transport).await
    }

    /// Connect to the system libvirt daemon.
    pub async fn connect_system() -> Result<Self> {
        Self::connect_unix(SYSTEM_SOCKET_PATH).await
    }

    /// Connect to the session libvirt daemon.
    pub async fn connect_session() -> Result<Self> {
        let runtime_dir =
            std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
        let path = format!("{}/{}", runtime_dir, SESSION_SOCKET_PATH);
        Self::connect_unix(&path).await
    }

    /// Create a connection from an existing transport.
    async fn from_transport<T: Transport + 'static>(transport: T) -> Result<Self> {
        let (tx, rx) = mpsc::channel::<WriteRequest>(32);

        let inner = Arc::new(ConnectionInner {
            serial: AtomicU32::new(1),
            tx,
            pending: Mutex::new(HashMap::new()),
        });

        // Spawn the I/O task
        let inner_clone = inner.clone();
        tokio::spawn(async move {
            if let Err(e) = io_task(transport, rx, inner_clone).await {
                eprintln!("libvirt connection I/O error: {}", e);
            }
        });

        Ok(Self { inner })
    }

    /// Make an RPC call.
    pub async fn call(&self, procedure: u32, payload: Bytes) -> Result<Bytes> {
        let serial = self.inner.serial.fetch_add(1, Ordering::SeqCst) as i32;
        let packet = Packet::new_call(procedure, serial, payload);

        // Create response channel
        let (tx, rx) = oneshot::channel();

        // Register pending request
        {
            let mut pending = self.inner.pending.lock().await;
            pending.insert(serial, tx);
        }

        // Send write request
        let write_req = WriteRequest {
            packet,
            response_tx: {
                // Dummy tx - response comes through pending map
                let (tx, _) = oneshot::channel();
                tx
            },
        };

        self.inner
            .tx
            .send(write_req)
            .await
            .map_err(|_| Error::ConnectionClosed)?;

        // Wait for response
        rx.await.map_err(|_| Error::ConnectionClosed)?
    }

    /// Make a typed RPC call with XDR serialization.
    pub async fn call_xdr<Req, Resp>(&self, procedure: u32, args: &Req) -> Result<Resp>
    where
        Req: serde::Serialize,
        Resp: serde::de::DeserializeOwned,
    {
        let payload = libvirt_xdr::to_bytes(args)?;
        let response = self.call(procedure, Bytes::from(payload)).await?;
        let result = libvirt_xdr::from_bytes(&response)?;
        Ok(result)
    }
}

/// Implement LibvirtRpc trait for Connection to enable generated API methods.
impl LibvirtRpc for Connection {
    async fn rpc_call(&self, procedure: u32, payload: Vec<u8>) -> std::result::Result<Vec<u8>, RpcError> {
        let response = self.call(procedure, Bytes::from(payload)).await
            .map_err(|e| RpcError::Transport(e.to_string()))?;
        Ok(response.to_vec())
    }
}

/// Background I/O task that handles reading and writing.
async fn io_task<T: Transport>(
    mut transport: T,
    mut write_rx: mpsc::Receiver<WriteRequest>,
    inner: Arc<ConnectionInner>,
) -> Result<()> {
    // For simplicity, we'll use a single task that alternates between reading and writing.
    // A more robust implementation would use split streams or select!.

    loop {
        tokio::select! {
            // Handle write requests
            Some(req) = write_rx.recv() => {
                let encoded = req.packet.encode();
                if let Err(e) = transport.send(&encoded).await {
                    // Notify the caller
                    let _ = req.response_tx.send(Err(e));
                    continue;
                }

                // Read the response
                match transport.recv().await {
                    Ok(data) => {
                        match Packet::decode(data) {
                            Ok(packet) => {
                                // Find and notify the pending request
                                let mut pending = inner.pending.lock().await;
                                if let Some(tx) = pending.remove(&packet.serial) {
                                    if packet.status == Status::Ok {
                                        let _ = tx.send(Ok(packet.payload));
                                    } else {
                                        let _ = tx.send(Err(Error::RemoteError(
                                            String::from_utf8_lossy(&packet.payload).to_string()
                                        )));
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to decode packet: {}", e);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("Failed to receive packet: {}", e);
                        break;
                    }
                }
            }
            else => break,
        }
    }

    Ok(())
}
