use log::trace;
use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};
use tonic::{Code, Request, Response, Status};

pub mod proto {
    tonic::include_proto!("fs");
}

pub use crate::proto::file_system_server::{FileSystem, FileSystemServer};
use crate::proto::{
    ExecRequest, ExecResponse, GetRequest, GetResponse, ListRequest, ListResponse, MkdirRequest,
    MkdirResponse, WriteRequest, WriteResponse,
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
    fn children(&self) -> Option<&BTreeMap<String, Tree<T>>> {
        match self {
            Tree::Leaf(_) => None,
            Tree::Parent(children) => Some(children),
        }
    }

    fn children_mut(&mut self) -> Option<&mut BTreeMap<String, Tree<T>>> {
        match self {
            Tree::Leaf(_) => None,
            Tree::Parent(children) => Some(children),
        }
    }

    fn get<P: IntoIterator<Item = String>>(&self, path: P) -> Option<&Tree<T>> {
        let mut node = self;
        for segment in path {
            node = node.children()?.get(&segment)?;
        }
        Some(node)
    }

    fn get_mut<P: IntoIterator<Item = String>>(&mut self, path: P) -> Option<&mut Tree<T>> {
        let mut node = self;
        for segment in path {
            node = node.children_mut()?.get_mut(&segment)?;
        }
        Some(node)
    }
}

#[tonic::async_trait]
impl FileSystem for FileSystemImpl {
    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
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

    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        trace!("[LIST] request = {:?}", request);
        let path = request.into_inner().path;
        let root = self.root.read().unwrap();
        let node = root
            .get(segments(&path))
            .ok_or(Status::new(Code::NotFound, "no such file"))?;
        let paths = match node {
            Tree::Leaf(_) => vec![path],
            Tree::Parent(files) => files
                .iter()
                .map(|(k, v)| match v {
                    Tree::Parent(_) => format!("{}/", k),
                    Tree::Leaf(_) => format!("{}", k),
                })
                .collect(),
        };
        Ok(Response::new(ListResponse { paths }))
    }

    async fn write(
        &self,
        request: Request<WriteRequest>,
    ) -> Result<Response<WriteResponse>, Status> {
        trace!("[WRITE] request = {:?}", request);
        let msg = request.into_inner();
        let path = msg.path;
        let content = msg.content;

        let mut segs = segments(&path);
        let name = segs
            .pop()
            .ok_or(Status::new(Code::InvalidArgument, "illegal filename"))?;

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
    ) -> Result<Response<MkdirResponse>, Status> {
        trace!("[MKDIR] request = {:?}", request);
        let msg = request.into_inner();
        let path = msg.path;

        let mut segs = segments(&path);
        let name = segs
            .pop()
            .ok_or(Status::new(Code::InvalidArgument, "illegal dirname"))?;

        match self.root.write().unwrap().get_mut(segs) {
            Some(Tree::Parent(files)) => {
                files
                    .entry(name)
                    .or_insert_with(|| Tree::Parent(BTreeMap::new()));
            }
            _ => return Err(Status::new(Code::NotFound, "no such directory")),
        };
        Ok(Response::new(MkdirResponse {}))
    }

    async fn exec(&self, request: Request<ExecRequest>) -> Result<Response<ExecResponse>, Status> {
        trace!("[EXEC] request = {:?}", request);
        let msg = request.into_inner();
        let wasm = msg.wasm;
        let imports = wasmer_runtime::imports!();
        let instance = wasmer_runtime::instantiate(&wasm, &imports)
            .map_err(|_| Status::new(Code::InvalidArgument, "invalid wasm"))?;
        let main: wasmer_runtime::Func<i32, i32> = instance.func("main").map_err(|_| {
            Status::new(
                Code::InvalidArgument,
                "could not find exported function 'main'",
            )
        })?;
        let n = main.call(42).map_err(|_| {
            Status::new(Code::FailedPrecondition, "execution error in provided wasm")
        })?;
        Ok(Response::new(ExecResponse { n }))
    }
}

fn segments(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|segment| !segment.is_empty())
        .map(String::from)
        .collect()
}
