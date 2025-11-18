fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile protobuf definitions
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(&["proto/registry.proto"], &["proto"])?;

    // Rerun build if proto files change
    println!("cargo:rerun-if-changed=proto/registry.proto");

    Ok(())
}
