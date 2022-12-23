use std::cell::UnsafeCell;

use bvh::{normalize, Point, BVH};
use gpu::wgpu::TextureFormat::Rgba8Uint;
use gpu::Context;
use gpu::Execution::PerPixel2D;
use rayon::prelude::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use ui::MainGui;
use window::{
    input::{button::ButtonState, Input},
    main_loop::{main_loop_run, Game, RenderResult, UpdateResult},
    winit::window::Window,
};

use crate::bvh::{cross, dot, Ray};

mod bvh;

pub struct HelloWorld {
    pub gpu_context: Context,
    pub ui: MainGui,
    pub bvh: BVH,
    pub time: f32,
    pub distance: f32,
    pub screen_buffer: Vec<[u8; 4]>,
}

const SCREEN_WIDTH: u32 = 1280;
const HALF_WIDTH: u32 = SCREEN_WIDTH / 2;
const SCREEN_HEIGHT: u32 = 720;
const HALF_HEIGHT: u32 = SCREEN_HEIGHT / 2;

impl Game for HelloWorld {
    fn on_init(window: &Window) -> Self {
        let mut gpu_context = Context::new(
            window,
            "to_draw_texture",
            "crates/CoGrRs/examples/ray_tracer/",
        );

        gpu_context.texture(
            "to_draw_texture",
            (SCREEN_WIDTH, SCREEN_HEIGHT, 1),
            gpu_context.config.format,
        );
        gpu_context.texture("depth_buffer", (SCREEN_WIDTH, SCREEN_HEIGHT, 1), Rgba8Uint);
        gpu_context.pipeline("draw", [], PerPixel2D);

        let screen_buffer = vec![[0; 4]; (SCREEN_WIDTH * SCREEN_HEIGHT) as usize];

        let mut bvh = BVH::construct("crates/CoGrRs/examples/ray_tracer/dragon.obj");
        bvh.build_bvh();

        let ui = MainGui::new(&gpu_context, window);

        HelloWorld {
            gpu_context,
            ui,
            bvh,
            time: 0f32,
            distance: 1f32,
            screen_buffer,
        }
    }

    fn on_render(&mut self, input: &mut Input, dt: f32, window: &Window) -> RenderResult {
        //self.time += 0.1;
        self.distance += input.mouse_state.scroll_delta;
        let ray_origin = Point::new(
            self.distance, //self.time.sin() * self.distance,
            0f32,
            0f32, // self.time.cos() * self.distance,
        );
        let ray_direction = normalize(&Point::new(-ray_origin.pos[0], 0f32, -ray_origin.pos[2]));
        let ray_side = cross(&ray_direction, &normalize(&Point::new(0f32, 1f32, 0f32)));
        let ray_up = cross(&ray_direction, &ray_side);
        self.screen_buffer = (0..SCREEN_HEIGHT * SCREEN_WIDTH)
            .into_iter()
            .map(|index| {
                let x = index % SCREEN_WIDTH;
                let y = index / SCREEN_WIDTH;

                let screen_point = ray_origin
                    + ray_direction
                    + ray_side * (x as f32 - HALF_WIDTH as f32)
                        / (SCREEN_WIDTH as f32 / (SCREEN_WIDTH as f32 / SCREEN_HEIGHT as f32))
                    + ray_up * (y as f32 - HALF_HEIGHT as f32) / SCREEN_HEIGHT as f32;

                let ray_direction = normalize(&(screen_point - ray_origin));
                let ray_r_direction = Point::new(
                    1f32 / ray_direction.pos[0],
                    1f32 / ray_direction.pos[1],
                    1f32 / ray_direction.pos[2],
                );
                let mut ray = Ray {
                    o: ray_origin,
                    d: ray_direction,
                    d_r: ray_r_direction,
                    t: f32::MAX,
                    prim: u32::MAX,
                    _padding1: 0,
                    _padding2: 0,
                };

                self.bvh.fast_intersect(&mut ray);

                /*return [
                    (ray.t) as u8, //(intensity * 255f32) as u8,
                    (ray.t) as u8, //(intensity * 255f32) as u8,
                    (ray.t) as u8, //(intensity * 255f32) as u8,
                    255,
                ];*/

                if ray.t < 10000000f32 {
                    let normal = self.bvh.triangle_normal(ray.prim);
                    let intensity =
                        (dot(&normal, &normalize(&Point::new(1f32, -1f32, 1f32))) + 1f32) / 10f32;

                    [
                        (intensity * 255f32) as u8,
                        (intensity * 255f32) as u8,
                        (intensity * 255f32) as u8,
                        255,
                    ]
                } else {
                    [0, 0, 0, 255]
                }
            })
            .collect(); //.collect_into_vec(&mut self.screen_buffer);

        self.gpu_context.set_texture_data(
            "depth_buffer",
            self.screen_buffer.as_slice(),
            (SCREEN_WIDTH, SCREEN_HEIGHT, 1),
        );
        let mut encoder = self.gpu_context.get_encoder_for_draw();

        self.gpu_context
            .dispatch_pipeline("draw", &mut encoder, &[0; 0]);

        self.gpu_context.image_buffer_to_screen(&mut encoder);

        self.ui.text("fps", &(1f32 / dt).to_string());

        self.ui.draw(
            &self.gpu_context,
            &mut encoder,
            window,
            input.mouse_state.mouse_location,
            input.mouse_state.get_left_button() == ButtonState::Pressed,
        );

        self.gpu_context.execute_encoder(encoder);
        RenderResult::Continue
    }

    fn on_resize(&mut self, _new_size: (u32, u32)) {}

    fn on_tick(&mut self, _dt: f32) -> UpdateResult {
        UpdateResult::Continue
    }
}

fn main() {
    main_loop_run::<HelloWorld>(SCREEN_WIDTH, SCREEN_HEIGHT, 10f32);
}
