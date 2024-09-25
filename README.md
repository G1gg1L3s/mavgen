# mavgen

This repo contains mavlink code generator for Rust alternative to https://github.com/mavlink/rust-mavlink. The generated code tries to be compatible as much as possible with `mavlink` crates.

The reasons to create it was to fix some bugs with the current rust-mavlink code generator and generally improve the code quality with more flexible architecture and many unit and integration tests.

There are still a number of differences in generated code from the rust-mavlink version, which are for now undocumented.
