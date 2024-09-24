#[cfg(feature = "mavgen-test")]
mod build {
    use std::path::Path;

    pub fn main() {
        let definitions_dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("mavlink/message_definitions/v1.0");

        let mut definitions = std::fs::read_dir(definitions_dir)
            .unwrap()
            .map(|maybe_entry| maybe_entry.map(|entry| entry.path()))
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        definitions.sort();

        let out_dir = std::env::var_os("OUT_DIR").unwrap();

        mavgen::generate_dir(&definitions, Path::new(&out_dir))
            .expect("failed to generate mavlink");

        for def in definitions {
            println!("cargo:rerun-if-changed={}", def.display());
        }
    }
}

fn main() {
    #[cfg(feature = "mavgen-test")]
    build::main();
}
