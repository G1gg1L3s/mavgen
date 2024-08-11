use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use hard_xml::XmlRead;

use crate::xml;

const MAX_INCLUDE_RECURSION: usize = 10;

pub trait World {
    fn read_file(&self, path: &Path) -> std::io::Result<String>;
    fn normalise_path(&self, path: &Path) -> std::io::Result<PathBuf>;
}

pub struct FsWorld;

impl World for FsWorld {
    fn read_file(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(path)
    }

    fn normalise_path(&self, path: &Path) -> std::io::Result<PathBuf> {
        std::fs::canonicalize(path)
    }
}

#[derive(Debug)]
pub struct MavlinkFile {
    pub mavlink: xml::Mavlink,
    pub normalised_includes: Vec<PathBuf>,
}

#[derive(Debug)]
pub enum Error {
    Io {
        err: std::io::Error,
        path: PathBuf,
    },
    Xml {
        err: hard_xml::XmlError,
        path: PathBuf,
        content: String,
    },
    RecursionLimitExceeded {
        stack: Vec<PathBuf>,
    },
    CycleDetected,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Io { path, .. } => write!(f, "IO error while opening {:?}", path),
            Error::Xml { path, .. } => write!(f, "XML error while parsing {:?}", path),
            Error::RecursionLimitExceeded { .. } => write!(f, "recursion limit exceeded"),
            Error::CycleDetected => write!(f, "inclusion cycle detected"),
        }
    }
}

impl std::error::Error for Error {}

pub struct Parser<W> {
    parsed: HashMap<PathBuf, MavlinkFile>,
    world: W,
    errors: Vec<Error>,
    max_include_recursion: usize,
    inclusion_stack: Vec<PathBuf>,
}

impl<W: World> Parser<W> {
    pub fn new(world: W) -> Self {
        Self {
            parsed: Default::default(),
            world,
            errors: Default::default(),
            max_include_recursion: MAX_INCLUDE_RECURSION,
            inclusion_stack: Vec::with_capacity(MAX_INCLUDE_RECURSION),
        }
    }

    fn try_parse_recursively(&mut self, path: PathBuf) -> Result<(), Error> {
        if self.parsed.contains_key(&path) {
            return Ok(());
        }

        let raw = match self.world.read_file(&path) {
            Ok(ok) => ok,
            Err(err) => {
                return Err(Error::Io { err, path });
            }
        };

        let mavlink = match xml::Mavlink::from_str(&raw) {
            Ok(ok) => ok,
            Err(err) => {
                return Err(Error::Xml {
                    err,
                    path,
                    content: raw,
                })
            }
        };

        let normalised_includes = mavlink
            .include
            .iter()
            .map(|include| {
                let include_path = path.with_file_name(include);
                self.world
                    .normalise_path(&include_path)
                    .map_err(|err| Error::Io {
                        err,
                        path: include_path,
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;

        self.parsed.insert(
            path.clone(),
            MavlinkFile {
                normalised_includes: normalised_includes.clone(),
                mavlink,
            },
        );

        for include in normalised_includes {
            self.parse_normalised(include);
        }

        Ok(())
    }

    fn parse_normalised(&mut self, normalised: PathBuf) {
        if self.inclusion_stack.len() == self.max_include_recursion {
            self.errors.push(Error::RecursionLimitExceeded {
                stack: self.inclusion_stack.clone(),
            });
            return;
        }

        self.inclusion_stack.push(normalised.clone());

        if let Err(err) = self.try_parse_recursively(normalised) {
            self.errors.push(err);
        }

        self.inclusion_stack.pop();
    }

    pub fn parse(&mut self, file: &Path) {
        match self.world.normalise_path(file) {
            Ok(ok) => self.parse_normalised(ok),
            Err(err) => self.errors.push(Error::Io {
                err,
                path: file.to_owned(),
            }),
        }
    }

    fn detect_cycles(&mut self) {
        let mut topo = topo_sort::TopoSort::with_capacity(self.parsed.len());
        for (path, file) in &self.parsed {
            if file.normalised_includes.contains(path) {
                self.errors.push(Error::CycleDetected);
                return;
            }

            topo.insert(path, file.normalised_includes.iter());
        }

        for node in &topo {
            if node.is_err() {
                self.errors.push(Error::CycleDetected);
                return;
            }
        }
    }

    pub fn finish(mut self) -> Result<HashMap<PathBuf, MavlinkFile>, Vec<Error>> {
        self.detect_cycles();

        if self.errors.is_empty() {
            Ok(self.parsed)
        } else {
            Err(self.errors)
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::HashMap;

    pub struct MockWorld(pub HashMap<PathBuf, String>);

    impl World for MockWorld {
        fn read_file(&self, path: &Path) -> std::io::Result<String> {
            self.0
                .get(path)
                .cloned()
                .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"))
        }

        fn normalise_path(&self, path: &Path) -> std::io::Result<PathBuf> {
            // simulate that we are working from some working dir
            if path.is_absolute() {
                Ok(normalize_path::NormalizePath::normalize(path))
            } else {
                let cwd = Path::new("/cwd/").join(path);
                Ok(normalize_path::NormalizePath::normalize(&*cwd))
            }
        }
    }

    #[test]
    fn test_one_file() {
        let world = MockWorld(HashMap::from_iter([(
            PathBuf::from("/cwd/test.xml"),
            String::from(
                r#"<?xml version="1.0"?>
                <mavlink>
                    <enums>
                        <enum name="TEST">
                            <entry name="TEST_A"/>
                            <entry name="TEST_B"/>
                            <entry name="TEST_C" value="10"/>
                        </enum>
                    </enums>

                    <messages/>
                </mavlink>
                "#,
            ),
        )]));

        let mut parser = Parser::new(world);

        parser.parse(Path::new("test.xml"));
        assert_eq!(parser.errors.len(), 0, "errors: {:?}", parser.errors);
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test.xml")),
            "parsed: {:?}",
            parser.parsed
        );

        let parsed = parser.finish().unwrap();
        assert!(
            parsed.contains_key(Path::new("/cwd/test.xml")),
            "parsed: {:?}",
            parsed
        );
    }

    #[test]
    fn test_self_import() {
        let world = MockWorld(HashMap::from_iter([(
            PathBuf::from("/cwd/test.xml"),
            String::from(
                r#"<?xml version="1.0"?>
                <mavlink>
                    <include>test.xml</include>
                    <enums>
                        <enum name="TEST">
                            <entry name="TEST_A"/>
                            <entry name="TEST_B"/>
                            <entry name="TEST_C" value="10"/>
                        </enum>
                    </enums>

                    <messages/>
                </mavlink>
                "#,
            ),
        )]));

        let mut parser = Parser::new(world);

        parser.parse(Path::new("test.xml"));
        assert_eq!(parser.errors.len(), 0, "errors: {:?}", parser.errors);
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test.xml")),
            "parsed: {:?}",
            parser.parsed
        );

        let err = parser.finish().unwrap_err();
        assert!(matches!(err[0], Error::CycleDetected));
    }

    #[test]
    fn test_loop2() {
        let world = MockWorld(HashMap::from_iter([
            (
                PathBuf::from("/cwd/test-1.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                <mavlink>
                    <include>test-2.xml</include>
                    <enums/>
                    <messages/>
                </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-2.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                <mavlink>
                    <include>test-1.xml</include>
                    <enums/>
                    <messages/>
                </mavlink>
                "#,
                ),
            ),
        ]));

        let mut parser = Parser::new(world);

        parser.parse(Path::new("test-1.xml"));
        assert_eq!(parser.errors.len(), 0, "errors: {:?}", parser.errors);
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test-1.xml")),
            "parsed: {:?}",
            parser.parsed
        );
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test-2.xml")),
            "parsed: {:?}",
            parser.parsed
        );

        let err = parser.finish().unwrap_err();
        assert!(matches!(err[0], Error::CycleDetected));
    }

    #[test]
    fn test_loop3() {
        let world = MockWorld(HashMap::from_iter([
            (
                PathBuf::from("/cwd/test-1.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-2.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-2.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-3.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-3.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-1.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
        ]));

        let mut parser = Parser::new(world);

        parser.parse(Path::new("/cwd/test-1.xml"));
        assert_eq!(parser.errors.len(), 0, "errors: {:?}", parser.errors);
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test-1.xml")),
            "parsed: {:?}",
            parser.parsed
        );
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test-2.xml")),
            "parsed: {:?}",
            parser.parsed
        );
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test-3.xml")),
            "parsed: {:?}",
            parser.parsed
        );

        let err = parser.finish().unwrap_err();
        assert!(matches!(err[0], Error::CycleDetected));
    }

    #[test]
    fn test_recursion_limit() {
        let world = MockWorld(HashMap::from_iter([
            (
                PathBuf::from("/cwd/test-1.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-2.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-2.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-3.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-3.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-4.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-4.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
        ]));

        let mut parser = Parser::new(world);
        parser.max_include_recursion = 3;

        parser.parse(Path::new("test-1.xml"));
        assert_eq!(parser.errors.len(), 1, "errors: {:?}", parser.errors);
        let Error::RecursionLimitExceeded { stack } = &parser.errors[0] else {
            panic!("err: {:?}", parser.errors[0]);
        };

        assert_eq!(
            stack,
            &[
                PathBuf::from("/cwd/test-1.xml"),
                PathBuf::from("/cwd/test-2.xml"),
                PathBuf::from("/cwd/test-3.xml"),
            ]
        );
    }

    #[test]
    fn test_multi_file() {
        let world = MockWorld(HashMap::from_iter([
            (
                PathBuf::from("/cwd/test-1.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>test-2.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-3.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-2.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
            (
                PathBuf::from("/cwd/test-4.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
        ]));

        let mut parser = Parser::new(world);

        parser.parse(Path::new("test-1.xml"));
        parser.parse(Path::new("test-3.xml"));
        assert_eq!(parser.errors.len(), 0, "errors: {:?}", parser.errors);
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test-1.xml")),
            "parsed: {:?}",
            parser.parsed
        );
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test-2.xml")),
            "parsed: {:?}",
            parser.parsed
        );
        assert!(
            parser.parsed.contains_key(Path::new("/cwd/test-3.xml")),
            "parsed: {:?}",
            parser.parsed
        );

        let parsed = parser.finish().unwrap();
        assert!(
            parsed.contains_key(Path::new("/cwd/test-1.xml")),
            "parsed: {:?}",
            parsed
        );
        assert!(
            parsed.contains_key(Path::new("/cwd/test-2.xml")),
            "parsed: {:?}",
            parsed
        );
        assert!(
            parsed.contains_key(Path::new("/cwd/test-3.xml")),
            "parsed: {:?}",
            parsed
        );
    }

    #[test]
    fn test_include_dir() {
        let world = MockWorld(HashMap::from_iter([
            (
                PathBuf::from("/dir1/test.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>../dir2/test-1.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
            (
                PathBuf::from("/dir2/test-1.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <include>/dir2/test-2.xml</include>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
            (
                PathBuf::from("/dir2/test-2.xml"),
                String::from(
                    r#"<?xml version="1.0"?>
                    <mavlink>
                        <enums/>
                        <messages/>
                    </mavlink>
                    "#,
                ),
            ),
        ]));

        let mut parser = Parser::new(world);

        parser.parse(Path::new("/dir1/test.xml"));
        assert_eq!(parser.errors.len(), 0, "errors: {:?}", parser.errors);
        assert!(
            parser.parsed.contains_key(Path::new("/dir1/test.xml")),
            "parsed: {:?}",
            parser.parsed
        );
        assert!(
            parser.parsed.contains_key(Path::new("/dir2/test-1.xml")),
            "parsed: {:?}",
            parser.parsed
        );
        assert!(
            parser.parsed.contains_key(Path::new("/dir2/test-2.xml")),
            "parsed: {:?}",
            parser.parsed
        );

        let parsed = parser.finish().unwrap();
        assert!(
            parsed.contains_key(Path::new("/dir1/test.xml")),
            "parsed: {:?}",
            parsed
        );
        assert!(
            parsed.contains_key(Path::new("/dir2/test-1.xml")),
            "parsed: {:?}",
            parsed
        );
        assert!(
            parsed.contains_key(Path::new("/dir2/test-2.xml")),
            "parsed: {:?}",
            parsed
        );
    }
}
