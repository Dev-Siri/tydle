use anyhow::Result;
use tydle::{Extract, Tydle, TydleOptions, VideoId, cookies::read_from_cookie_file};

#[tokio::main]
async fn main() -> Result<()> {
    tydle::logger::init_logging("info");

    let ty = Tydle::new(TydleOptions {
        auth_cookies: read_from_cookie_file("./target/cookies.txt")?,
        ..Default::default()
    })?;

    ty.get_streams(&VideoId::new("j5t0zuVD-zg")?).await?;

    Ok(())
}
