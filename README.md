# fraud_detection 5

> **Warning:** The `.cargo/` folder contains Windows-specific configuration (custom `target-dir` for OneDrive, CPU flags). Delete or rename before building:
> ```bash
> mv .cargo .cargo.bak
> ```
> More information on this [page](https://www.40tude.fr/docs/06_programmation/rust/005_my_rust_setup_win11/my_rust_setup_win11.html#onedrive).


## Description

First tests with speckit and SDD.


## Installation

```bash
cargo build
cargo build --release
```

## Usage

```bash
$env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG
cargo run

```

## Testing

```bash
cargo test
```

## License

MIT License - see [LICENSE](LICENSE) for details


## Contributing
This project is developed for personal and educational purposes. Feel free to explore and use it to enhance your own learning.

Given the nature of the project, external contributions are not actively sought nor encouraged. However, constructive feedback aimed at improving the project (in terms of speed, accuracy, comprehensiveness, etc.) is welcome. Please note that this project is being created as a hobby and is unlikely to be maintained once my initial goal has been achieved.
