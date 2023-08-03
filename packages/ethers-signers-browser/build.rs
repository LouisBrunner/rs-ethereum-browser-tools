use std::{env, path::Path};
use trunk::{cmd::build::Build, config::ConfigOptsBuild};

const FRONTEND: &str = "../ethers-signers-browser-frontend";

#[tokio::main]
async fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    println!("cargo:rerun-if-changed={}", FRONTEND);
    Build {
        build: ConfigOptsBuild {
            release: true,
            public_url: Some("/dist/".to_string()),
            target: Some(Path::new(FRONTEND).join("index.html").to_path_buf()),
            dist: Some(
                Path::new(env::var("OUT_DIR").unwrap().as_str()).join("frontend").to_path_buf(),
            ),
            ..ConfigOptsBuild::default()
        },
    }
    .run(None)
    .await
    .unwrap();
}
