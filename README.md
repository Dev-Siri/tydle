# ty

> NOTE: This project is yet to be finished.

`ty` is an extremely small subset of `yt-dlp`, written entirely in Rust. Unlike `yt-dlp` and all the video-downloaders based around or on it, `ty` is meant to be minimal and provide a developer-facing API. It only fetches streams from YouTube and parses the player to return the URL or player signature.

The purpose of `ty` is not to be used as a CLI application or just as a Rust library, but to be ran on any platform, focused primarily on the client. This partly explains why its so minimal. It can be used in web-based projects through WebAssembly. It's also usable in other languages, e.g. Go, Swift, and , with the FFI bindings.

## Getting Started

Clone the repository.

```
$ git clone https://github.com/Dev-Siri/ty
```

## License

This project is [MIT](LICENSE) licensed.
