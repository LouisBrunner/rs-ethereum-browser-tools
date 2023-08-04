use std::path::Path;
use trunk_build_time::{cmd::build::Build, config::ConfigOptsBuild};

const FRONTEND: &str = "../ethers-signers-browser-frontend";

async fn build_frontend(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed={}", path);
    Build {
        build: ConfigOptsBuild {
            release: true,
            public_url: Some("/dist/".to_string()),
            target: Some(Path::new(path).join("index.html").to_path_buf()),
            dist: Some(
                Path::new(std::env::var("OUT_DIR").expect("OUT_DIR not set").as_str()).join("frontend").to_path_buf(),
            ),
            ..ConfigOptsBuild::default()
        },
    }
    .run(None)
    .await?;
  Ok(())
}

#[tokio::main]
async fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    match build_frontend(FRONTEND).await {
      Err(e) => {
        eprintln!("Failed to build frontend, fallback to versionned: {}", e);
        // FIXME: we shouldn't assume the frontend will have the same version
        let frontend_vers = format!("{}-{}", FRONTEND, std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION not set"));
        build_frontend(&frontend_vers).await.unwrap();
      },
      _ => {},
    };
}
