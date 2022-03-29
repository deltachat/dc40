# dc40

dc40 has no active development atm, we rather focuse on [dreamer](https://github.com/dignifiedquire/dreamer) which is a pure rust experimental desktop client for deltachat using egui.

## Development Dependencies

- [`rustup`](https://rustup.rs/)
- [`trunk`](https://trunkrs.dev/)

- rust target `wasm32-unknown-unknown`
```sh
$ rustup target add wasm32-unknown-unknown
```

- [`cargo-tauri`](https://tauri.studio/)

```sh
$ cargo install tauri-cli --version 1.0.0-beta.5
```

- [`trunk`](https://trunkrs.dev/)
```sh
$ cargo install --locked trunk
```

## Development Running


```sh
$ cargo tauri dev
```
