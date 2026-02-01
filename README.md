🌲 Redwood

Redwood is a TUI (Text User Interface) that turns your shell into a flight tracking station.

### Features
- **Different Modes:** A single-panel "spotter" mode that shows the current closest aircraft to you, plus a more detailed dashboard with a sorted list of closest aircraft. 
- **Auto-Geolocation:** Automatically geolocates to you and shows your current area. This can be turned off and you can configure a custom (or more exact) coordinate target area via `config.toml`.

### ⚙️ Custom Configuration
Redwood TUI generates a `config.toml` on first run. You can customize your experience by editing this file:

- `auto_gpu`: Set to `false` to use manual home coordinates.
- `detection_radius`: How far (in km) to look for planes. (Default: 50km)
- `poll_interval_seconds`: How often to refresh data. (Default: 30s)

## Docker
If you don't want to install the Rust toolchain, you can run Redwood via Docker:

```bash
docker run -it --rm nihcosnosaj/redwood-tui