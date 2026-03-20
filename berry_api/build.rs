fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile proto files
    tonic_build::configure()
        .build_server(true)
        .build_client(false)
        .compile_protos(
            &[
                "../berrycode/proto/berry_api.proto",
            ],
            &["../berrycode/proto"],
        )?;
    Ok(())
}
