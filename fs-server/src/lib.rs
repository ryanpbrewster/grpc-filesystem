use log::trace;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use tonic::{Code, Request, Response, Status};

pub mod proto {
    tonic::include_proto!("fs");
}

pub use crate::proto::file_system_server::FileSystemServer;
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
    pub fn into_service(self) -> proto::file_system_server::FileSystemServer<Self> {
        proto::file_system_server::FileSystemServer::new(self)
    }
}

#[tonic::async_trait]
impl proto::file_system_server::FileSystem for FileSystemImpl {
    async fn get(
        &self,
        request: Request<GetRequest>,
    ) -> Result<Response<GetResponse>, tonic::Status> {
        trace!("[GET] request = {:?}", request);
        let path = request.into_inner().path;
        let response: GetResponse = match self.root.read().unwrap().get(segments(&path)) {
            Some(Tree::Leaf(content)) => GetResponse {
                content: content.clone(),
            },
            _ => return Err(Status::new(Code::NotFound, "no such file")),
        };
        Ok(Response::new(response))
    }

    async fn list(
        &self,
        request: Request<ListRequest>,
    ) -> Result<Response<ListResponse>, tonic::Status> {
        trace!("[LIST] request = {:?}", request);
        let path = request.into_inner().path;
        let response: ListResponse = match self.root.read().unwrap().get(segments(&path)) {
            None => return Err(Status::new(Code::NotFound, "no such file")),
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
        Ok(Response::new(response))
    }

    async fn write(
        &self,
        request: Request<WriteRequest>,
    ) -> Result<Response<WriteResponse>, tonic::Status> {
        trace!("[WRITE] request = {:?}", request);
        let msg = request.into_inner();
        let path = msg.path;
        let content = msg.content;

        let mut segs = segments(&path);
        let name = match segs.pop() {
            None => return Err(Status::new(Code::InvalidArgument, "illegal filename")),
            Some(name) => name,
        };

        match self.root.write().unwrap().get_mut(segs) {
            Some(Tree::Parent(files)) => {
                if let Some(Tree::Parent(_)) = files.get(&name) {
                    return Err(Status::new(
                        Code::InvalidArgument,
                        "cannot write to directory",
                    ));
                }
                files.insert(name, Tree::Leaf(content));
            }
            _ => return Err(Status::new(Code::NotFound, "no such directory")),
        };
        Ok(Response::new(WriteResponse {}))
    }

    async fn mkdir(
        &self,
        request: Request<MkdirRequest>,
    ) -> Result<Response<MkdirResponse>, tonic::Status> {
        trace!("[MKDIR] request = {:?}", request);
        let msg = request.into_inner();
        let path = msg.path;

        let mut segs = segments(&path);
        let name = match segs.pop() {
            None => return Err(Status::new(Code::InvalidArgument, "illegal dirname")),
            Some(name) => name,
        };

        match self.root.write().unwrap().get_mut(segs) {
            Some(Tree::Parent(files)) => {
                if let Some(Tree::Parent(_)) = files.get(&name) {
                    return Err(Status::new(Code::InvalidArgument, "file exists"));
                }
                files.insert(name, Tree::Parent(BTreeMap::new()));
            }
            _ => return Err(Status::new(Code::NotFound, "no such directory")),
        };
        Ok(Response::new(MkdirResponse {}))
    }
}

fn segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(String::from)
        .collect()
}
