# Quickstart: Concurrent Pipeline

**Branch**: `004-concurrent-pipeline`

## Run (infinite mode -- press CTRL+C to stop)

```powershell
$env:RUST_LOG='info'; cargo run; Remove-Item env:RUST_LOG
```

Expected output: log lines from Producer (`producer.batch.written`) and Consumer
(`consumer.batch.processed`) interleave in real time. Press CTRL+C to stop cleanly.

## Run (debug -- per-transaction output)

```powershell
$env:RUST_LOG='debug'; cargo run; Remove-Item env:RUST_LOG
```

## Run tests (all 55 must pass)

```powershell
cargo test --workspace
```

## Demo mode (finite iterations -- exits automatically)

Set `iterations(10)` in `ProducerConfig` in `main.rs` to run a bounded demo:
```rust
let producer_config = ProducerConfig::builder(100)
    .iterations(10)
    .poll_interval1(Duration::from_millis(50))
    .build()?;
```

## Shutdown behavior

| Scenario | What happens |
|---|---|
| CTRL+C | buffer1.close() called; select! arm fires; main returns Ok(()) |
| Producer finishes (finite) | buffer1.close() called by wrapper; Consumer drains remaining data; join! resolves |
