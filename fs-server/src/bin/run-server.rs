use log::info;
use tonic::transport::Server;

use fs_server::{FileSystemImpl, FileSystemServer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let addr = "[::1]:50051".parse().unwrap();
    let server = FileSystemImpl::default();
    info!("listening at {:?}", addr);
    Server::builder()
        .add_service(FileSystemServer::new(server))
        .serve(addr)
        .await?;

    Ok(())
}
