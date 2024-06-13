# led_matrix_monitoring

This service collects metrics and renders them on Framework 16" LED matrix using [LED Matrix Daemon](https://github.com/night-crawler/led_matrix_daemon).
All in all, it just renders a PNG image with the metrics and sends it to the daemon responsible for rendering it on LED matrix.

## Features

Metric collectors:

- [x] CPU usage
- [x] Memory % usage
- [x] Disk IO usage
- [x] Network usage
- [x] CPU Temperature
- [x] Battery Level

Widgets:
 - [x] Network/disk plot
 - [x] Temperature bar
 - [x] Battery level bar
 - [x] CPU usage bar per core + average
 - [x] Memory usage bar

## Installation

### Arch Linux

```bash
yay -S led_matrix_monitoring
```

Enable daemon with default configuration:

```bash
sudo systemctl enable --now led_matrix_monitoring.service
```

### Build

Install Rust:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Check out the repository and build the binary:

```bash
git clone https://github.com/night-crawler/led_matrix_monitoring.git
cd led_matrix_monitoring
cargo build --release
```

Copy the binary to a location in your path:

```bash
sudo cp ./target/release/led_matrix_monitoring /usr/local/bin
```

## Configuration

Take a look at [example_config.toml](example_config.toml).

In the collector section, everything that takes a list of values will produce an average of those values.
You might want to change widget position here and there.
