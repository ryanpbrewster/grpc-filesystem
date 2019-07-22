use futures::{future, Future, Stream};
use log::{error, trace, info};
use tokio::net::TcpListener;
use tower_grpc::{Request, Response};
use tower_hyper::server::{Http, Server};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/fs.rs"));
}

use crate::proto::server;
use crate::proto::{GetRequest, GetResponse,
                   ListRequest, ListResponse,
                   WriteRequest, WriteResponse};

#[derive(Clone, Debug)]
struct FileSystemImpl;

impl server::FileSystem for FileSystemImpl {
    type GetFuture = future::FutureResult<Response<GetResponse>, tower_grpc::Status>;
    type ListFuture = future::FutureResult<Response<ListResponse>, tower_grpc::Status>;
    type WriteFuture = future::FutureResult<Response<WriteResponse>, tower_grpc::Status>;

    fn get(&mut self, request: Request<GetRequest>) -> Self::GetFuture {
      trace!("[GET] request = {:?}", request);

        let response = Response::new(GetResponse {
            content: b"Zomg, it works!".to_vec(),
        });

        future::ok(response)
    }

    fn list(&mut self, request: Request<ListRequest>) -> Self::ListFuture {
      trace!("[LIST] request = {:?}", request);
      unimplemented!()
    }

    fn write(&mut self, request: Request<WriteRequest>) -> Self::WriteFuture {
      trace!("[WRITE] request = {:?}", request);
      unimplemented!()
    }
}

pub fn main() {
    let _ = ::env_logger::init();

    let new_service = server::FileSystemServer::new(FileSystemImpl);

    let mut server = Server::new(new_service);

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
