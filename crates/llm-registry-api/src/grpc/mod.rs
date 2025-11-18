// ! gRPC API implementation
//!
//! This module provides a complete gRPC API for the LLM Registry using Tonic.
//! It supports all registry operations including streaming for real-time updates.

pub mod converters;
pub mod service;

// Include the generated protobuf code
pub mod proto {
    tonic::include_proto!("llm.registry.v1");
}

pub use proto::registry_service_server::{RegistryService, RegistryServiceServer};
pub use service::RegistryServiceImpl;

use tonic::transport::Server;
use std::net::SocketAddr;

/// Build a gRPC server with the registry service
pub fn build_grpc_server(
    service: RegistryServiceImpl,
) -> tonic::transport::server::Router {
    Server::builder().add_service(RegistryServiceServer::new(service))
}

/// Serve the gRPC server on the specified address
pub async fn serve_grpc(
    addr: SocketAddr,
    service: RegistryServiceImpl,
) -> Result<(), Box<dyn std::error::Error>> {
    Server::builder()
        .add_service(RegistryServiceServer::new(service))
        .serve(addr)
        .await?;
    Ok(())
}
