# ty

`ty` is an extremely small subset of `yt-dlp`, written entirely in Rust. Unlike `yt-dlp` and all the video-downloaders based around it or on it, `ty` is meant to be minimal. It only fetches streams from YouTube on WASM environments, and provides an additional module for deciphering of player signatures on native platforms. The purpose of `ty` is not to be used as a CLI application or a Rust library, but to be ran on any platform, focused primarily on the client. This partly explains why its so minimal. It can be used in web-based projects through WebAssembly. Usage in languages other than Rust (Go, Swift, or something else) is possible with FFI.

## Getting Started

## License

This project is [MIT](LICENSE) licensed.
