use inline_spirv_runtime::{ShaderCompilationConfig, ShaderKind};
use regex::Regex;
use rspirv_reflect::PushConstantInfo;

use crate::Execution;

pub struct Shader {
    pub config: ShaderCompilationConfig,
    pub shader: Vec<u32>,
    pub push_constant_info: PushConstantInfo,
    pub cg_x: u32, //compute group size x
    pub cg_y: u32,
    pub cg_z: u32,
    pub bindings: Vec<String>,
}

impl Shader {
    pub fn get_shader_properties(shader_name: &str, shaders_folder: &str) -> Shader {
        let mut config = inline_spirv_runtime::ShaderCompilationConfig::default();
        config.debug = true;
        config.kind = ShaderKind::Compute;
        let shader_file = shaders_folder.to_string() + shader_name + ".comp";

        let shader_vec: Vec<u32> = inline_spirv_runtime::runtime_compile(
            &std::fs::read_to_string(&shader_file).unwrap_or_else(|_| panic!("Could not find {}", shader_file)),
            Some(&(shader_file)),
            &config,
        )
        .map_err(|e| println!("{}", e))
        .unwrap_or_else(|_| panic!("could not compile shader: {}", shader_file));

        let shader: &[u8] = unsafe { std::slice::from_raw_parts(shader_vec.as_ptr() as *const u8, shader_vec.len() * 4) };
        let reflector = rspirv_reflect::Reflection::new_from_spirv(shader).unwrap_or_else(|_| panic!("could not reflect shader: {}", shader_file));
        let push_constant_info = match reflector
            .get_push_constant_range()
            .unwrap_or_else(|_| panic!("could not get push constant range from shader: {}", shader_file))
        {
            Some(p) => p,
            None => PushConstantInfo { offset: 0, size: 0 },
        };
        let compute_group_sizes = reflector
            .get_compute_group_size()
            .unwrap_or_else(|| panic!("could not get compute group size from shader: {}", shader_file));

        let text = reflector.disassemble();

        let re = Regex::new(r"buffer [^\s\\]*_block|(([ui]*image3D|[ui]*image2D|[ui]*image1D) [a-z_A-Z]*)").expect("somehow couldnt compile regex");
        let bindings: Vec<String> = re
            .find_iter(&text)
            .map(|val| val.as_str().split(' ').collect::<Vec<&str>>()[1].to_string())
            .collect::<Vec<String>>();

        Shader {
            config,
            shader: shader_vec,
            cg_x: compute_group_sizes.0,
            cg_y: compute_group_sizes.1,
            cg_z: compute_group_sizes.2,
            bindings,
            push_constant_info,
        }
    }
}

pub fn get_execution_dims(workgroup_size: (u32, u32, u32), execution_mode: Execution, texture_size: (u32, u32)) -> (u32, u32, u32) {
    match execution_mode {
        Execution::PerPixel1D => ((texture_size.0 * texture_size.1 + workgroup_size.0 - 1) / workgroup_size.0, 1u32, 1u32),
        Execution::PerPixel2D => (
            (texture_size.0 + workgroup_size.0 - 1) / workgroup_size.0,
            (texture_size.1 + workgroup_size.1 - 1) / workgroup_size.1,
            1,
        ),
        Execution::N3D(n) => (
            (n + workgroup_size.0 - 1) / workgroup_size.0,
            (n + workgroup_size.1 - 1) / workgroup_size.1,
            (n + workgroup_size.2 - 1) / workgroup_size.2,
        ),
        Execution::N1D(n) => ((n + workgroup_size.0 - 1) / workgroup_size.0, 1, 1),
    }
}
