use criterion::{criterion_group, criterion_main, Criterion};
use tempdir::TempDir;
use std::path::{Path, PathBuf};
use anyhow::{Context, Error};
use hotg_rune_cli::run::{
    Image,
    image::ImageSource,
    Sound,
    sound::AudioClip,
    new_multiplexer,
};

use hotg_rune_wasmer_runtime::Runtime;
use hotg_runicos_base_runtime::BaseImage;
use hotg_rune_core::capabilities;

use hotg_rune_codegen::{
    Compilation, DefaultEnvironment, RuneProject, Verbosity,
};

use hotg_rune_syntax::{hir::Rune, yaml::Document, Diagnostics};

// TODO: Refactor this out as this is shared with tests
pub fn project_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .canonicalize()
        .unwrap();

    for ancestor in manifest_dir.ancestors() {
        if ancestor.join(".git").is_dir() {
            return ancestor.to_path_buf();
        }
    }

    unreachable!(
        "Unable to determine the project's root directory. Where is \".git/\"?"
    );
}

pub fn example_dir() -> PathBuf { project_root().join("examples") }

fn load_rune(path: PathBuf) -> Vec<u8> {
    std::fs::read(&path).with_context(|| {
        format!("Unable to read \"{}\"", path.display())
    }).unwrap()
}

fn parse_runefile(runefile: &Path) -> Result<Rune, Error> {
    let src = std::fs::read_to_string(runefile).with_context(|| {
        format!("Unable to read \"{}\"", runefile.display())
    }).unwrap();

    let mut diags = Diagnostics::new();
    let parsed = Document::parse(&src).unwrap();
    let rune = hotg_rune_syntax::analyse_yaml_runefile(&parsed, &mut diags);

    if diags.has_errors() {
        anyhow::bail!("Aborting compilation due to errors.");
    }

    Ok(rune)
}

fn build_rune(rune_path: &PathBuf, rune_name: String, rune: Rune) {
    let rune_build_dir = TempDir::new("rune_build_dir").unwrap();

    let compilation = Compilation {
        name: rune_name,
        rune: rune,
        working_directory: rune_build_dir.path().to_path_buf(),
        current_directory: rune_path.clone(),
        rune_project: RuneProject::Disk(project_root()),
        optimized: true,
        verbosity: Verbosity::Quiet,
    };

    let mut env = DefaultEnvironment::for_compilation(&compilation);

    let blob = hotg_rune_codegen::generate_with_env(compilation, &mut env)
        .context("Rune compilation failed").unwrap();

    assert!(blob.len() != 0);

    rune_build_dir.close().expect("Unable to close the rune build directory");
}

fn sine_build_benchmark(c: &mut Criterion) {
    let base = example_dir().join("sine");
    let rune = parse_runefile(base.join("Runefile.yml").as_path()).unwrap();

    c.bench_function("sine_build",
        |b| b.iter(|| { build_rune(&base, String::from("sine"), rune.clone()) }));
}

fn sine_inference_benchmark(c: &mut Criterion) {
    let mut runtime = Runtime::load(&load_rune(example_dir().join("sine").join("sine.rune")),
                                    BaseImage::with_defaults())
        .context("Unable to create rune runtime")
        .unwrap();

    c.bench_function("sine_inference",
        |b| b.iter(|| { runtime.call().context("Call Failed").unwrap() }));
}

fn microspeech_inference_benchmark(c: &mut Criterion) {
    let base = example_dir().join("microspeech");

    let mut img = BaseImage::with_defaults();
    let wav_file = base.join("data").join("right").join("fb7eb481_nohash_0.wav");
    img.register_capability(
        capabilities::SOUND,
        new_multiplexer::<Sound, _>(vec![AudioClip::from_wav_file(wav_file).unwrap()]));

    let mut runtime = Runtime::load(&load_rune(base.join("microspeech.rune")), img)
        .context("Unable to create rune runtime")
        .unwrap();

    c.bench_function("microspeech_inference",
        |b| b.iter(|| runtime.call().context("Call Failed").unwrap()));
}

fn styletransfer_inference_benchmark(c: &mut Criterion) {
    let base = example_dir().join("style_transfer");

    let mut img = BaseImage::with_defaults();
    img.register_capability(
        capabilities::IMAGE,
        new_multiplexer::<Image, _>(vec![ImageSource::from_file(base.join("style.jpg")).unwrap(),
                                         ImageSource::from_file(base.join("flower.jpg")).unwrap()]));

    let mut runtime = Runtime::load(&load_rune(base.join("style_transfer.rune")), img)
        .context("Unable to create rune runtime")
        .unwrap();

    c.bench_function("styletransfer_inference",
        |b| b.iter(|| runtime.call().context("Call Failed").unwrap()));
}

criterion_group!(
    name = build_benchmark;
    config = Criterion::default().significance_level(0.1).sample_size(10);
    targets = sine_build_benchmark);

criterion_group!(
    name = inference_benchmark;
    config = Criterion::default().significance_level(0.1).sample_size(10);
    targets = sine_inference_benchmark, microspeech_inference_benchmark, styletransfer_inference_benchmark);

criterion_main!(build_benchmark, inference_benchmark);
