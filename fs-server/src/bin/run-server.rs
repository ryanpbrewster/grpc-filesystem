use futures::{Future, Stream};
use log::{error, info};
use tokio::net::TcpListener;
use tower_hyper::server::{Http, Server};

use fs_server::FileSystemImpl;

pub fn main() {
    let _ = ::env_logger::init();

    let mut server = Server::new(FileSystemImpl::default().into_service());

    let http = Http::new().http2_only(true).clone();

    let addr = "[::1]:50051".parse().unwrap();
    let bind = TcpListener::bind(&addr).expect("bind");
    info!("listening at {:?}", addr);

    let serve = bind
        .incoming()
        .for_each(move |sock| {
            if let Err(e) = sock.set_nodelay(true) {
                return Err(e);
            }

            let serve = server.serve_with(sock, http.clone());
            tokio::spawn(serve.map_err(|e| error!("hyper error: {:?}", e)));

            Ok(())
        })
        .map_err(|e| eprintln!("accept error: {}", e));

    tokio::run(serve)
}
