fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile(
            &[
                "../../proto/aegis/v1/common.proto",
                "../../proto/aegis/v1/sidecar.proto",
                "../../proto/aegis/v1/gateway.proto",
                "../../proto/aegis/v1/control.proto",
            ],
            &["../../proto"],
        )?;
    Ok(())
}
