# mavgen-test

This crate contains integration tests with pymavlink. It compiles definitions from the `mavlink` repo as part of the build.rs process.

All crate functionality is hidden behind the `mavgen-test` feature to avoid building definitions by IDE when a project is open. TO run the tests, you should have `python3` and `python3-venv` installed. Then, run the tests:

```
cargo test --package mavgen-test --features mavgen-test,all-dialects
```

The test work as following:

The crate provides a binary which accepts a dialect and starts a TCP server that accepts mavlink messages and pongs them back to the peer.

Under the `tests`, you will find rust integration tests that prepare python environment (initialize venv, install pymavlink, compile definitions and run the python test). The python test is located at `tests/mavtest.py`. It accepts the binary and a dialect, runs the crate binary, forwards all available messages and verifies that they are replayed correctly.
