use std::{env, path::PathBuf};

// This is called by `cargo build` as a pre-build step
// to generate the rust files corresponding to the proto files
// using the library tonic
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .file_descriptor_set_path(out_dir.join("sample_descriptor.bin"))
        .compile(&["src/api.proto"], &["src"])?;
    // TODO: For production, remove the above two lines to remove the reflection endpoint
    // and instead use the below line to compile
    //tonic_build::compile_protos("src/api.proto")?;
    Ok(())
}
