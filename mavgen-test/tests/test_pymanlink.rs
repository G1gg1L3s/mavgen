#![cfg(feature = "mavgen-test")]

use std::{
    path::{Path, PathBuf},
    println,
    process::Command,
};

#[allow(unused)]
fn definitions() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("mavlink/message_definitions/v1.0")
}

fn venv_dir() -> PathBuf {
    let out_dir = std::env::var_os("OUT_DIR").unwrap();
    PathBuf::from(out_dir).join("venv")
}

fn mavgen_py() -> PathBuf {
    venv_dir().join("bin/mavgen.py")
}

fn pip() -> PathBuf {
    venv_dir().join("bin/pip")
}

pub fn python() -> PathBuf {
    venv_dir().join("bin/python")
}

pub fn init_python() {
    static INIT_PY: std::sync::Once = std::sync::Once::new();

    // This should be done only once before all testcases
    INIT_PY.call_once(|| {
        println!("Initialising python");

        let venv = venv_dir();

        let status = Command::new("python3")
            .arg("-m")
            .arg("venv")
            .arg(&venv)
            .status()
            .expect("failed to create virtualenv");
        assert!(status.success());

        let status = Command::new(pip())
            .args(["install", "pymavlink~=2.4"])
            .status()
            .expect("failed to install pymavlink");
        assert!(status.success());
    })
}

pub fn compile_mavlink(def: &Path) -> PathBuf {
    let out = std::env::var_os("OUT_DIR").unwrap();
    let out = PathBuf::from(out)
        .join("py")
        .join(def.file_name().unwrap())
        .with_extension("py");

    let base = out.parent().unwrap();
    std::fs::create_dir_all(base).unwrap();

    let status = Command::new(mavgen_py())
        .arg("--lang=Python3")
        .arg("--wire-protocol=2.0")
        .arg("--no-validate") // TODO: why doesn't it work without it?
        .arg("--output")
        .arg(out.clone())
        .arg(def)
        .status()
        .expect("failed to generate mavlink dialect");

    assert!(status.success());

    out
}

#[allow(unused)]
fn test_dialect(dialect_xml: &str) {
    init_python();
    let mavgen_test_bin = assert_cmd::cargo::cargo_bin("mavgen-test");

    let dialect_module = compile_mavlink(&definitions().join(dialect_xml));
    let test_dir = Path::new(env!("CARGO_MANIFEST_DIR"));

    let status = Command::new(python())
        .arg(test_dir.join("tests/mavtest.py"))
        .arg("--dialect")
        .arg(dialect_module)
        .arg("--mavgen-test")
        .arg(mavgen_test_bin)
        .status()
        .unwrap();

    assert!(status.success());
}

#[test]
#[cfg(feature = "all")]
fn test_all() {
    test_dialect("all.xml");
}

#[test]
#[cfg(feature = "ardupilotmega")]
fn test_ardupilotmega() {
    test_dialect("ardupilotmega.xml");
}

#[test]
#[cfg(feature = "asluav")]
fn test_asluav() {
    test_dialect("ASLUAV.xml");
}

#[test]
#[cfg(feature = "avssuas")]
fn test_avssuas() {
    test_dialect("AVSSUAS.xml");
}

#[test]
#[cfg(feature = "common")]
fn test_common() {
    test_dialect("common.xml");
}

#[test]
#[cfg(feature = "cubepilot")]
fn test_cubepilot() {
    test_dialect("cubepilot.xml");
}

#[test]
#[cfg(feature = "development")]
fn test_development() {
    test_dialect("development.xml");
}

#[test]
#[cfg(feature = "matrixpilot")]
fn test_matrixpilot() {
    test_dialect("matrixpilot.xml");
}

#[test]
#[cfg(feature = "paparazzi")]
fn test_paparazzi() {
    test_dialect("paparazzi.xml");
}

#[test]
#[cfg(feature = "storm32")]
fn test_storm32() {
    test_dialect("storm32.xml");
}

#[test]
#[cfg(feature = "u_avionix")]
fn test_u_avionix() {
    test_dialect("uAvionix.xml");
}

#[test]
#[cfg(feature = "ualberta")]
fn test_ualberta() {
    test_dialect("ualberta.xml");
}
