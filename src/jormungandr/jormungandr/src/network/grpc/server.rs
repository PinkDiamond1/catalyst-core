use super::super::{service::NodeService, Channels, GlobalStateR};
use crate::settings::start::network::Listen;
use network_grpc::server::{self, Server};
use tokio::prelude::*;

pub fn run_listen_socket(
    listen: Listen,
    state: GlobalStateR,
    channels: Channels,
) -> impl Future<Item = (), Error = ()> {
    let sockaddr = listen.address();

    info!(
        state.logger(),
        "start listening and accepting gRPC connections on {}", sockaddr
    );

    match server::listen(&sockaddr) {
        Err(error) => {
            error!(
                state.logger(),
                "Error while listening on {}: {}", sockaddr, error
            );
            unimplemented!()
        }
        Ok(listener_stream) => {
            let fold_logger = state.logger().clone();
            let err_logger = state.logger().clone();
            let node_server = NodeService::new(channels, state);
            let server = Server::new(node_server);

            listener_stream
                .map_err(move |err| {
                    // error while receiving an incoming connection
                    // here we might need to log the error and try
                    // to listen again on the sockaddr
                    error!(
                        err_logger,
                        "Error while accepting connection on {}: {:?}", sockaddr, err
                    );
                })
                .fold(server, move |mut server, stream| {
                    // received incoming connection
                    let conn_logger =
                        fold_logger.new(o!("peer_addr" => stream.peer_addr().unwrap()));
                    info!(
                        conn_logger,
                        "incoming P2P connection on {}",
                        stream.local_addr().unwrap(),
                    );

                    let conn = server.serve(stream);
                    tokio::spawn(conn.map_err(move |e| {
                        use network_grpc::server::Error;

                        match e {
                            Error::Protocol(e) => {
                                info!(conn_logger, "incoming P2P HTTP/2 connection error"; "reason" => %e)
                            }
                            _ => {
                                warn!(conn_logger, "incoming P2P connection failed"; "error" => ?e);
                            }
                        }
                    }));

                    future::ok(server)
                })
                .map(|_| ())
        }
    }
}
