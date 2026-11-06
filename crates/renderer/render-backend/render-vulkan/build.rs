use std::env;
use std::fs;
use std::path::PathBuf;

fn compile(source: &str, stage: naga::ShaderStage, entry: &str) -> Result<Vec<u32>, String> {
    let module = naga::front::wgsl::parse_str(source)
        .map_err(|err| format!("failed to parse WGSL: {err:?}"))?;

    let mut validator = naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::all(),
    );
    let info = validator
        .validate(&module)
        .map_err(|err| format!("failed to validate WGSL: {err:?}"))?;

    let options = naga::back::spv::Options {
        lang_version: (1, 0),
        ..Default::default()
    };
    let mut writer = naga::back::spv::Writer::new(&options)
        .map_err(|err| format!("failed to create SPIR-V writer: {err:?}"))?;

    let pipeline_options = naga::back::spv::PipelineOptions {
        shader_stage: stage,
        entry_point: entry.to_owned(),
    };

    let mut words = Vec::new();
    writer
        .write(&module, &info, Some(&pipeline_options), &None, &mut words)
        .map_err(|err| format!("failed to generate SPIR-V: {err:?}"))?;

    Ok(words)
}

fn main() {
    println!("cargo:rerun-if-changed=shaders/triangle.wgsl");

    let source = fs::read_to_string("shaders/triangle.wgsl").expect("failed to read triangle.wgsl");
    let vertex = compile(&source, naga::ShaderStage::Vertex, "vs_main").expect("vertex shader");
    let fragment =
        compile(&source, naga::ShaderStage::Fragment, "fs_main").expect("fragment shader");

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let generated = format!(
        "pub const VERTEX_SPV: &[u32] = &{vertex:?};\npub const FRAGMENT_SPV: &[u32] = &{fragment:?};\n"
    );

    fs::write(out_dir.join("triangle_shaders.rs"), generated)
        .expect("failed to write generated shader module");
}
