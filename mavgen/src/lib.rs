use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub mod codegen;
pub mod flatten;
pub mod model;
pub mod normaliser;
pub mod parser;
pub mod xml;

use parser::World;

#[derive(Debug)]
pub enum Error {
    CreateDir(std::io::Error, PathBuf),
    ParseXml(Vec<parser::Error>),
    NormalisePath(std::io::Error, PathBuf),
    Flattening(std::io::Error),
    Normalisation(Vec<normaliser::Error>, PathBuf),
    InvalidFilename(OsString),
    WritingToFile(std::io::Error, PathBuf),
}

pub fn generate_dir(files: &[PathBuf], out_dir: &Path) -> Result<(), Error> {
    std::fs::create_dir_all(out_dir).map_err(|err| Error::CreateDir(err, out_dir.to_path_buf()))?;

    let mut parser = parser::Parser::new(parser::FsWorld);
    for file in files {
        parser.parse(file);
    }

    let parsed = parser.finish().map_err(Error::ParseXml)?;

    let mut modules = vec![];

    for file in files {
        let normalised = parser::FsWorld
            .normalise_path(file)
            .map_err(|err| Error::NormalisePath(err, file.to_path_buf()))?;

        let module = flatten::flatten(&parsed, &normalised).map_err(Error::Flattening)?;

        modules.push(module);
    }

    let mut normalised_modules = vec![];

    for (module, path) in modules.into_iter().zip(files) {
        let normaliser = normaliser::Normaliser::default();
        let normalised = normaliser
            .normalise_module(module)
            .map_err(|err| Error::Normalisation(err, path.to_path_buf()))?;
        normalised_modules.push(normalised);
    }

    let codegen = codegen::rust::Codegen::default();
    let mut mod_codegen = codegen::rust::ModCodegen::default();

    for module in normalised_modules {
        let module_name = module.path.file_stem().expect("path should be a file");
        let module_name = module_name
            .to_str()
            .ok_or_else(|| Error::InvalidFilename(module_name.to_os_string()))?;

        let module_name = codegen::rust::naming::snake_case(module_name);

        let mut new_path = out_dir.join(Path::new(&module_name));
        new_path.set_extension("rs");

        let stream = codegen.emit_module(&module);
        // TODO: dump raw stream to a temp file for debugging
        let ast = syn::parse2(stream).expect("stream must be correct");
        let formatted = prettyplease::unparse(&ast);
        std::fs::write(&new_path, formatted).map_err(|err| Error::WritingToFile(err, new_path))?;

        mod_codegen.add_mod(&module_name);
    }

    let stream = mod_codegen.finish();
    // TODO: dump raw stream to a temp file for debugging
    let ast = syn::parse2(stream).expect("stream must be correct");
    let formatted = prettyplease::unparse(&ast);
    let mod_path = out_dir.join(Path::new("mod.rs"));
    std::fs::write(&mod_path, formatted).map_err(|err| Error::WritingToFile(err, mod_path))?;

    Ok(())
}

pub fn generate_one(input: &Path, output: &Path) -> Result<(), Error> {
    let mut parser = parser::Parser::new(parser::FsWorld);
    parser.parse(input);
    let parsed = parser.finish().map_err(Error::ParseXml)?;

    let normalised = parser::FsWorld
        .normalise_path(input)
        .map_err(|err| Error::NormalisePath(err, input.to_path_buf()))?;

    let module = flatten::flatten(&parsed, &normalised).map_err(Error::Flattening)?;

    let module_name = module.path.file_stem().expect("path should be a file");
    let module_name = module_name
        .to_str()
        .ok_or_else(|| Error::InvalidFilename(module_name.to_os_string()))?;
    let module_name = codegen::rust::naming::snake_case(module_name);

    let normaliser = normaliser::Normaliser::default();
    let normalised = normaliser
        .normalise_module(module)
        .map_err(|err| Error::Normalisation(err, input.to_path_buf()))?;

    let codegen = codegen::rust::Codegen::default();

    let output = if output.is_file() {
        output.to_path_buf()
    } else {
        output.join(format!("{}.rs", module_name))
    };

    let stream = codegen.emit_module(&normalised);
    // TODO: dump raw stream to a temp file for debugging
    let ast = syn::parse2(stream).expect("stream must be correct");
    let formatted = prettyplease::unparse(&ast);
    std::fs::write(&output, formatted)
        .map_err(|err| Error::WritingToFile(err, output.to_path_buf()))?;

    Ok(())
}
