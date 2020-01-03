use crate::{
    gen::node::server as gen_server,
    service::{protocol_bounds, NodeService},
};

use network_core::server::{BlockService, FragmentService, GossipService, Node};

use futures::prelude::*;
use tokio_io::{AsyncRead, AsyncWrite};
use tokio_tcp::{TcpListener, TcpStream};
use tower_grpc::codegen::server::grpc::Never as NeverError;
use tower_hyper::server::Http;

#[cfg(unix)]
use tokio_uds::{UnixListener, UnixStream};

use std::io;
use std::net::SocketAddr;

#[cfg(unix)]
use std::os::unix::net::SocketAddr as UnixSocketAddr;
#[cfg(unix)]
use std::path::Path;

/// The gRPC server for the blockchain node.
///
/// This type encapsulates the gRPC protocol server providing the
/// Node service. The application instantiates a `Server` wrapping a
/// blockchain service implementation satisfying the abstract network
/// service trait `Node`.
pub struct Server<T>
where
    T: Node + Clone,
    <T::BlockService as BlockService>::Block: protocol_bounds::Block,
    <T::BlockService as BlockService>::Header: protocol_bounds::Header,
    <T::FragmentService as FragmentService>::Fragment: protocol_bounds::Fragment,
    <T::GossipService as GossipService>::Node: protocol_bounds::Node,
{
    inner: tower_hyper::Server<
        gen_server::NodeServer<NodeService<T>>,
        gen_server::node::ResponseBody<NodeService<T>>,
    >,
    http: Http,
}

/// The error type for gRPC server operations.
pub type Error = tower_hyper::server::Error<NeverError>;

/// Connection of a client peer to the gRPC server.
pub struct Connection {
    inner: tower_hyper::server::Serve<NeverError>,
}

impl Future for Connection {
    type Item = ();
    type Error = Error;

    #[inline]
    fn poll(&mut self) -> Poll<(), Error> {
        self.inner.poll()
    }
}

impl<T> Server<T>
where
    T: Node + Clone + Send + 'static,
    <T::BlockService as BlockService>::Block: protocol_bounds::Block,
    <T::BlockService as BlockService>::Header: protocol_bounds::Header,
    <T::FragmentService as FragmentService>::Fragment: protocol_bounds::Fragment,
    <T::GossipService as GossipService>::Node: protocol_bounds::Node,
{
    /// Creates a server instance around the node service implementation.
    pub fn new(node: T) -> Self {
        let grpc_service = gen_server::NodeServer::new(NodeService::new(node));
        let inner = tower_hyper::Server::new(grpc_service);
        let mut http = Http::new();
        http.http2_only(true);
        Server { inner, http }
    }

    /// Initializes a client peer connection based on an accepted connection
    /// socket. The socket can be obtained from a stream returned by `listen`.
    pub fn serve<S>(&mut self, sock: S) -> Connection
    where
        S: AsyncRead + AsyncWrite + Send + 'static,
    {
        Connection {
            inner: self.inner.serve_with(sock, self.http.clone()),
        }
    }
}

/// Sets up a listening TCP socket bound to the given address.
/// If successful, returns an asynchronous stream of `TcpStream` socket
/// objects representing accepted TCP connections from clients.
/// The TCP_NODELAY option is disabled on the returned sockets as
/// necessary for the HTTP/2 protocol.
pub fn listen(addr: &SocketAddr) -> Result<TcpListen, io::Error> {
    let inner = TcpListener::bind(&addr)?;
    Ok(TcpListen { inner })
}

/// Sets up a listening Unix socket bound to the specified path.
/// If successful, returns an asynchronous stream of `UnixStream` socket
/// objects representing accepted connections from clients.
#[cfg(unix)]
pub fn listen_unix<P: AsRef<Path>>(
    path: P,
) -> Result<impl Stream<Item = UnixStream, Error = io::Error>, io::Error> {
    let listener = UnixListener::bind(path)?;
    Ok(listener.incoming())
}

// Returns `Ok` if the error is per-connection, meaning that it's still
// possible to listen and accept connections on the same socket
// after this error. Otherwise, returns the error.
// Code inspired by crate tk-listen under the terms of
// Apache-2.0 and MIT licenses.
fn handle_accept_error(e: io::Error) -> io::Result<()> {
    use io::ErrorKind::*;

    match e.kind() {
        ConnectionAborted | ConnectionReset | ConnectionRefused => Ok(()),
        #[cfg(target_os = "macos")]
        InvalidInput => Ok(()),
        _ => Err(e),
    }
}

// Handles errors occurring on setting a socket option.
// Mac OS requires special treatment because on this platform, EINVAL
// can occur in case the socket has been remotely disconnected.
// We ignore this and let the connection die later rather than reporting
// a listener error.
fn handle_setsockopt_error(e: io::Error) -> io::Result<()> {
    match e.kind() {
        #[cfg(target_os = "macos")]
        io::ErrorKind::InvalidInput => Ok(()),
        _ => Err(e),
    }
}

pub struct TcpListen {
    inner: TcpListener,
}

impl Stream for TcpListen {
    type Item = (TcpStream, SocketAddr);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, io::Error> {
        loop {
            match self.inner.poll_accept() {
                Ok(Async::Ready((sock, addr))) => {
                    sock.set_nodelay(true).or_else(handle_setsockopt_error)?;
                    return Ok(Async::Ready(Some((sock, addr))));
                }
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => {
                    handle_accept_error(e)?;
                }
            }
        }
    }
}

#[cfg(unix)]
pub struct UnixListen {
    inner: UnixListener,
}

#[cfg(unix)]
impl Stream for UnixListen {
    type Item = (UnixStream, UnixSocketAddr);
    type Error = io::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, io::Error> {
        loop {
            match self.inner.poll_accept() {
                Ok(Async::Ready((sock, addr))) => {
                    return Ok(Async::Ready(Some((sock, addr))));
                }
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(e) => {
                    handle_accept_error(e)?;
                }
            }
        }
    }
}
