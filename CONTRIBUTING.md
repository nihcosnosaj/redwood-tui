# CONTRIBUTING.md

Hello! Thank you for taking the time to contribute to this effort to bring some aviation nerdiness to the terminal.

### Development environment

Of course, you'll need a few things before contributing:
1. Rust Toolchain: the latest stable version of Rust installed on your local machine.
2. A second terminal window to monitor logs from the app.

To get started:
1. Fork the repo and clone it locally
2. Create a feature branch via `git checkout -b feature/cool-new-feature`
3. Make sure the project builds via `cargo build`


### Principles 

I stuck to some principles while building this out, mainly:
1. We want to keep `src/ui.rs` (the UI view) stateless. It's sole concern is rendering the current `App` state. There should be zero network calls or heavy business logic inside the `draw` loop.
2. All I/O (calls to OpenSky, writing of log files) must happen async via `tokio::spawn`. The main thread is for the TUI loop.
3. The terminal is a shared resource. Always use the provided panic hooks and ensure `restore_terminal()` is called on exit.

### Quality

We want to keep the code in this repo super clean. While we have a robust CI pipeline, it's good practice to also run these before submitting a PR:
1. Formatting: `cargo fmt --all`
2. Linting: `cargo clippy -- -D warnings`
3. Testing: `cargo test`

##### Remember, this is a TUI
Just a friendly reminder, since this is a TUI, we don't want to print directly to stdout via `println!`. We are using the `tracing` macros (`info!`, `debug!`, `error!`).

During development, you can see logs in real-time via running the app in one window with `cargo run` and in a second, seperate window: `tail -f logs/redwood.log`

### PR Submission
As always, follow best practices for commits and submitting pull requests. These include:
1. Keeping your commits small and focused (atomic!)
2. Explain nature of change adnn why in description. If the UI changed at all, a small gif or screenshot is always helpful.
3. If a new feature is added, make sure the README reflects any impactful changes.
