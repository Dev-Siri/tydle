# tydle

`tydle` is an extremely small subset of `yt-dlp`, written entirely in Rust. Unlike `yt-dlp` and all the video-downloaders based around or on it, `tydle` is meant to be minimal and provide a developer-facing API. It provides a heavily modular approach to extract video metadata, streams or the raw manifest from YouTube and a separate module for deciphering signatures.

The purpose of `tydle` is not to be used as a CLI application or just as a Rust library, but to be ran on any platform, focused primarily on the client. It can be used in web-based projects through WebAssembly to run it on serverless functions and in other languages, like Go or Swift, with its FFI bindings.

## Usage

Require the crate in your `Cargo.toml` file:

```toml
tydle = "0.1.0"
```

Then use the crate in your Rust code:

```rs
use anyhow::Result;
// `Tydle` is the public interface to interact with YouTube.
use tydle::{Tydle, Extract};

async fn main() -> Result<()> {
  // Initialize `tydle`
  let ty = Tydle::new()?;

  // Now you can fetch depending on what you need.
  let manifest = ty.get_manifest(...).await?;
  let streams = ty.get_streams(...).await?;
  let video_info = ty.get_video_info(...).await?;

  Ok(())
}
```

### Managing Streams, Metadata and Manifests

`tydle`, with its `Extract` trait, provides two fetch functions, those being `get_streams` and `get_video_info`.

```rs
use anyhow::Result;
use tydle::{Tydle, Extract};

#[tokio::main]
async fn main() -> Result<()> {
  let ty = Tydle::new()?;

  let streams = ty.get_streams(...).await?;
  let video_info = ty.get_video_info(...).await?;

  Ok(())
}
```

These two functions need to fetch from YouTube's API to get the JSON object (termed the "manifest") which contains data about the video and streams. When you call these two functions specifically, they fetch the manifest themselves individually and return the parsed response directly.

This works great if you are only dealing with singular types of data, like only streams or only video metadata. **However, You should avoid using these functions if you need to fetch both the streams as well as the video metadata of the same video**, because they each fetch the manifest individually, so you'd be getting and parsing the manifest twice for the same video, which is just a waste of CPU cycles.

Instead, you should use the `get_manifest` method **to fetch the manifest once**, then you can pass it to one of the `x_from_manifest` functions to parse them as structs you can actually work with.

```rs
use anyhow::Result;
use tydle::{Tydle, VideoId, Extract};

#[tokio::main]
async fn main() -> Result<()> {
  let ty = Tydle::new()?;

  let video_id = VideoId::new("XDjB9E3YtUE")?;
  // Now that you have this manifest fetched once, simply pass it to the `x_from_manifest` functions.
  let manifest = ty.get_manifest(&video_id).await?;

  let video_info = ty.get_video_info_from_manifest(&manifest).await?;
  let streams = ty.get_streams_from_manifest(&manifest).await?;

  Ok(())
}
```

### Pitfalls Of Using WASM For Browsers

Since this library is focused for execution on client environments, you might be tempted to use the WebAssembly build for running it in the browser directly. However, even though you can get this `tydle` to run in the browser correctly, you won't be able to do anything useful other than signature deciphering. This happens because in the browser, CORS restrictions are imposed, preventing any fetches to YouTube's API from being possible.

The library can't work around this issue, even with a proxy option because then the streams fetched won't even be useful to you, as they are only available to the client that fetched it, which in the case of a proxy would be the proxy server and not the browser that's running `tydle`. Considering that extracted streams on the client being directly accessible from the client is a core focus of the library, it's useless if the browser imposes a restriction.

However, to make up for this, you can probably create a serverless function with the help of the WASM build. Since serverless functions (like on Vercel) can run WebAssembly and produce a reasonable response time, you could probably do something similar as shown below: (This example is using SvelteKit.)

```ts
import { error } from "@sveltejs/kit";
import { Tydle } from "tydle";

export async function GET({ params: { videoId } }) {
  try {
    const tydle = new Tydle();
    const streams = await tydle.fetchStreams(videoId);
    const urlStreams = streams.filter((stream) => "URL" in stream.source);

    // Since the streams are only accessible here,
    // you need to fetch the source URL here and send that video response back.
    return fetch(urlStreams.source[0]);
  } catch (err) {
    console.error(err);
    return error(500, "Failed to get video");
  }
}
```

Though this is slower than if the ability to directly call on the client was possible, it at least makes it usable on web environments.

### Signature Deciphering

Signature deciphering requires executing JavaScript somehow, as we need to execute YouTube's `player.js` file which contains the actual logic to decipher signatures.
`tydle` uses the [Deno](https://deno.com) JavaScript Runtime to decipher YouTube URL signatures on native platforms. In WebAssembly builds, it uses the `eval()` function from the JavaScript context to perform the action instead.

## Developing Locally

Clone the repository.

```sh
$ git clone https://github.com/Dev-Siri/tydle
```

You need to have Rust installed obviously.

Compile for native builds:

```sh
$ cargo build --release
```

Install `wasm-pack` if you are compiling for WebAssembly:

```sh
$ cargo install wasm-pack
```

After which, you can build with `wasm-pack` for target `wasm32-unknown-unknown` with an environment:

```sh
$ wasm-pack build --target bundler
```

## Credits

- [yt-dlp](https://github.com/yt-dlp/yt-dlp) for documentation of the YouTube APIs and providing the EJS modules.
- [youtube_explode_dart](https://github.com/Hexer10/youtube_explode_dart) for the implementation example of the Deno runtime for signature deciphering.

## License

This project is [MIT](LICENSE) licensed.
