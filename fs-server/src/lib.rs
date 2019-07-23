use futures::future;
use log::trace;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use tower_grpc::{Code, Request, Response, Status};

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/fs.rs"));
}

use crate::proto::{
    GetRequest, GetResponse, ListRequest, ListResponse, MkdirRequest, MkdirResponse, WriteRequest,
    WriteResponse,
};

#[derive(Clone, Debug)]
pub struct FileSystemImpl {
    root: Arc<RwLock<Tree<Vec<u8>>>>,
}
impl Default for FileSystemImpl {
    fn default() -> Self {
        FileSystemImpl {
            root: Arc::new(RwLock::new(Tree::Parent(BTreeMap::new()))),
        }
    }
}

#[derive(Debug)]
enum Tree<T> {
    Leaf(T),
    Parent(BTreeMap<String, Tree<T>>),
}
impl<T> Tree<T> {
    fn get<P: IntoIterator<Item = String>>(&self, path: P) -> Option<&Tree<T>> {
        let mut node = self;
        for segment in path {
            match node {
                Tree::Leaf(_) => return None,
                Tree::Parent(children) => match children.get(&segment) {
                    None => return None,
                    Some(child) => {
                        node = child;
                    }
                },
            }
        }
        Some(node)
    }

    fn get_mut<P: IntoIterator<Item = String>>(&mut self, path: P) -> Option<&mut Tree<T>> {
        let mut node = self;
        for segment in path {
            match node {
                Tree::Leaf(_) => return None,
                Tree::Parent(children) => match children.get_mut(&segment) {
                    None => return None,
                    Some(child) => {
                        node = child;
                    }
                },
            }
        }
        Some(node)
    }
}

impl FileSystemImpl {
    pub fn into_service(self) -> proto::server::FileSystemServer<Self> {
        proto::server::FileSystemServer::new(self)
    }
}

impl proto::server::FileSystem for FileSystemImpl {
    type GetFuture = future::FutureResult<Response<GetResponse>, tower_grpc::Status>;
    type ListFuture = future::FutureResult<Response<ListResponse>, tower_grpc::Status>;
    type WriteFuture = future::FutureResult<Response<WriteResponse>, tower_grpc::Status>;
    type MkdirFuture = future::FutureResult<Response<MkdirResponse>, tower_grpc::Status>;

    fn get(&mut self, request: Request<GetRequest>) -> Self::GetFuture {
        trace!("[GET] request = {:?}", request);
        let path = request.into_inner().path;
        let response: GetResponse = match self.root.read().unwrap().get(segments(&path)) {
            Some(Tree::Leaf(content)) => GetResponse {
                content: content.clone(),
            },
            _ => return future::err(Status::new(Code::NotFound, "no such file")),
        };
        future::ok(Response::new(response))
    }

    fn list(&mut self, request: Request<ListRequest>) -> Self::ListFuture {
        trace!("[LIST] request = {:?}", request);
        let path = request.into_inner().path;
        let response: ListResponse = match self.root.read().unwrap().get(segments(&path)) {
            None => return future::err(Status::new(Code::NotFound, "no such file")),
            Some(node) => match node {
                Tree::Leaf(_) => ListResponse { paths: vec![path] },
                Tree::Parent(files) => ListResponse {
                    paths: files
                        .iter()
                        .map(|(k, v)| match v {
                            Tree::Parent(_) => format!("{}/", k),
                            Tree::Leaf(_) => format!("{}", k),
                        })
                        .collect(),
                },
            },
        };
        future::ok(Response::new(response))
    }

    fn write(&mut self, request: Request<WriteRequest>) -> Self::WriteFuture {
        trace!("[WRITE] request = {:?}", request);
        let msg = request.into_inner();
        let path = msg.path;
        let content = msg.content;

        let mut segs = segments(&path);
        let name = match segs.pop() {
            None => return future::err(Status::new(Code::InvalidArgument, "illegal filename")),
            Some(name) => name,
        };

        match self.root.write().unwrap().get_mut(segs) {
            Some(Tree::Parent(files)) => {
                if let Some(Tree::Parent(_)) = files.get(&name) {
                    return future::err(Status::new(
                        Code::InvalidArgument,
                        "cannot write to directory",
                    ));
                }
                files.insert(name, Tree::Leaf(content));
            }
            _ => return future::err(Status::new(Code::NotFound, "no such directory")),
        };
        future::ok(Response::new(WriteResponse {}))
    }

    fn mkdir(&mut self, request: Request<MkdirRequest>) -> Self::MkdirFuture {
        trace!("[MKDIR] request = {:?}", request);
        let msg = request.into_inner();
        let path = msg.path;

        let mut segs = segments(&path);
        let name = match segs.pop() {
            None => return future::err(Status::new(Code::InvalidArgument, "illegal dirname")),
            Some(name) => name,
        };

        match self.root.write().unwrap().get_mut(segs) {
            Some(Tree::Parent(files)) => {
                if let Some(Tree::Parent(_)) = files.get(&name) {
                    return future::err(Status::new(Code::InvalidArgument, "file exists"));
                }
                files.insert(name, Tree::Parent(BTreeMap::new()));
            }
            _ => return future::err(Status::new(Code::NotFound, "no such directory")),
        };
        future::ok(Response::new(MkdirResponse {}))
    }
}

fn segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(String::from)
        .collect()
}
