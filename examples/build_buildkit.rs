//! Builds a container with a bunch of extra options for testing

use bollard::image::{BuildImageOptions, BuilderVersion};
use bollard::Docker;

use futures_util::stream::StreamExt;

use std::io::Write;

#[tokio::main]
async fn main() {
    let docker = Docker::connect_with_unix_defaults().unwrap();

    let dockerfile = String::from(
        "FROM alpine as builder1
RUN touch bollard.txt
FROM alpine as builder2
RUN --mount=type=bind,from=builder1,target=mnt cp mnt/bollard.txt buildkit-bollard.txt
ENTRYPOINT ls buildkit-bollard.txt
",
    );

    let mut header = tar::Header::new_gnu();
    header.set_path("Dockerfile").unwrap();
    header.set_size(dockerfile.len() as u64);
    header.set_mode(0o755);
    header.set_cksum();
    let mut tar = tar::Builder::new(Vec::new());
    tar.append(&header, dockerfile.as_bytes()).unwrap();

    let uncompressed = tar.into_inner().unwrap();
    let mut c = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    c.write_all(&uncompressed).unwrap();
    let compressed = c.finish().unwrap();

    let id = "bollard-build-buildkit-example";
    let build_image_options = BuildImageOptions {
        t: id,
        dockerfile: "Dockerfile",
        version: BuilderVersion::BuilderBuildKit,
        pull: true,
        #[cfg(feature = "buildkit")]
        session: Some(String::from(id)),
        ..Default::default()
    };

    let mut image_build_stream =
        docker.build_image(build_image_options, None, Some(compressed.into()));

    while let Some(Ok(bollard::models::BuildInfo { aux: Some(aux), .. })) =
        image_build_stream.next().await
    {
        println!("Response: {:?}", aux);
    }
}
