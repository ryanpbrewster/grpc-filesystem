use futures::future;
use log::trace;
use tower_grpc::{Request, Response};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/fs.rs"));
}

use crate::proto::{
    GetRequest, GetResponse, ListRequest, ListResponse, WriteRequest, WriteResponse,
};

#[derive(Clone, Debug)]
pub struct FileSystemImpl;
impl FileSystemImpl {
    pub fn into_service(self) -> proto::server::FileSystemServer<Self> {
        proto::server::FileSystemServer::new(self)
    }
}

impl proto::server::FileSystem for FileSystemImpl {
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
