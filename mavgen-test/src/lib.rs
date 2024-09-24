#![cfg(feature = "mavgen-test")]

pub mod messages {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}
