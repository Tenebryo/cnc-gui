use cgmath::*;
use image::flat::View;
use imgui::TextureId;

use vulkano::{buffer::{cpu_pool::CpuBufferPoolChunk, view}, command_buffer::PrimaryAutoCommandBuffer, format::ClearValue, image::{ImageCreateFlags, view::ImageView}, memory::pool::StdMemoryPool, sampler::Sampler};
use vulkano::pipeline::input_assembly::PrimitiveTopology;
use vulkano::command_buffer::DynamicState;
use vulkano::buffer::CpuBufferPool;
use vulkano::buffer::BufferUsage;
use vulkano::command_buffer::SubpassContents;
use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::{image::StorageImage, pipeline::GraphicsPipeline};
use vulkano::render_pass::Subpass;
use vulkano::render_pass::RenderPass;
use std::sync::Arc;

use vulkano::{impl_vertex, pipeline::GraphicsPipelineAbstract};

use vulkano::format::Format;
use crate::viewport::Viewport;

pub mod line_fs {vulkano_shaders::shader!{ty: "fragment",path: "src/shaders/line.frag",               include: [],}}
pub mod line_vs {vulkano_shaders::shader!{ty: "vertex",  path: "src/shaders/line.vert",               include: [],}}

use crate::simulation::{MotionPoint, MotionType};
use crate::imgui_renderer::System;

#[derive(Debug, Default, Clone, Copy)]
pub struct Vertex{
    pub pos : [f32;3],
    pub col : [f32;4],
    pub time : f32,
}

impl_vertex!(Vertex, pos, col, time);

pub struct GCodeRenderer {
    pub pipeline : Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub render_pass : Arc<RenderPass>,
    pub image : Option<Arc<StorageImage>>,
    pub vertex_pool : CpuBufferPool<Vertex>,
    pub vertex_buffer : Option<Arc<CpuBufferPoolChunk<Vertex, Arc<StdMemoryPool>>>>,
    pub texture_id : Option<TextureId>,
}

impl GCodeRenderer {
    pub fn init(system : &System) -> Self {
        let render_pass = Arc::new(
            vulkano::ordered_passes_renderpass!(system.device.clone(),
                attachments: {
                    depth: {
                        load: Clear,
                        store: DontCare,
                        format: Format::D16Unorm,
                        samples: 4,
                    },
                    msaa: {
                        load: Clear,
                        store: DontCare,
                        format: Format::R8G8B8A8Unorm,
                        samples: 4,
                    },
                    color: {
                        load: DontCare,
                        store: Store,
                        format: Format::R8G8B8A8Unorm,
                        samples: 1,
                    }
                },
                passes: [
                    {
                        color: [msaa],
                        depth_stencil: {depth},
                        input : [],
                        resolve:[color]
                    }
                ]
            )
            .unwrap(),


        );



        let line_fs = line_fs::Shader::load(system.device.clone()).expect("failed to create shader module");
        let line_vs = line_vs::Shader::load(system.device.clone()).expect("failed to create shader module");

        let pipeline = Arc::new(
            GraphicsPipeline::start()
                .vertex_input_single_buffer::<Vertex>()
                .vertex_shader(line_vs.main_entry_point(), ())
                .primitive_topology(PrimitiveTopology::LineList)
                .viewports_dynamic_scissors_irrelevant(1)
                // .depth_stencil_simple_depth()
                .depth_write(true)
                .line_width_dynamic()
                .blend_alpha_blending()
                .fragment_shader(line_fs.main_entry_point(), ())
                .render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
                .build(system.device.clone())
                .unwrap(),
        );

        let vertex_pool = CpuBufferPool::<Vertex>::new(system.device.clone(), BufferUsage::all());

        GCodeRenderer {
            render_pass,
            pipeline,
            image : None,
            vertex_pool,
            vertex_buffer : None,
            texture_id : None,
        }
    }

    pub fn render(&mut self, system : &mut System, viewport : &Viewport, cmd_buf_builder : &mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>, tmatrix : Matrix4<f32>, width : u32, height : u32) {

        let framebuffer = viewport.create_framebuffer(self.render_pass.clone());

        if let Some(framebuffer) = framebuffer {

            cmd_buf_builder.begin_render_pass(
                framebuffer, 
                SubpassContents::Inline, 
                // vec![1.0.into(), [0.0, 0.0, 0.0, 1.0].into()]
                vec![1.0.into(), [0.9, 0.9, 0.9, 1.0].into(), ClearValue::None]
            ).expect("failed to start render pass");


            if let Some(ref vb) = self.vertex_buffer {

                let ds = DynamicState {
                    viewports : Some(vec![vulkano::pipeline::viewport::Viewport {
                        origin : [0.0; 2],
                        dimensions : [width as f32, height as f32],
                        depth_range : 0.0..1.0,
                    }]),
                    line_width: Some(3.0),
                    ..DynamicState::none()
                };

                cmd_buf_builder.draw(
                    self.pipeline.clone(), &ds, vec![vb.clone()], (), 
                        line_vs::ty::PushConstants {
                            matrix : tmatrix.into(),
                            viewport : [width as f32, height as f32],
                        },
                        vec![]
                    )
                    .expect("failed to draw line");
            }

            cmd_buf_builder.end_render_pass()
                .expect("Failed to finish render pass");

        }
    }

    pub fn create_line_buffer(&mut self, motion_path : &[MotionPoint]) {

        let mut path = Vec::with_capacity(motion_path.len() * 2 - 2);

        for [p0, p1] in motion_path.array_windows::<2>() {

            let col = match p0.ty {
                MotionType::Rapid  => {[1.0, 0.1, 0.0, 1.0]}
                MotionType::Linear => {[0.0, 0.4, 1.0, 1.0]}
            };

            path.extend_from_slice(&[
                Vertex {
                    pos : (p0.pos).into(),
                    // col : [0.2, 0.2, 0.2, 1.0],
                    col,
                    time : p0.time,
                },
                Vertex {
                    pos : (p1.pos).into(),
                    // col : [0.2, 0.2, 0.2, 1.0],
                    col,
                    time : p1.time,
                },
            ]);
        }

        let new_vb = Arc::new(
            self.vertex_pool.chunk(path).expect("failed to allocated vertex buffer")
        );

        self.vertex_buffer = Some(new_vb);

    }

    pub fn clear_line_buffer(&mut self) {

        self.vertex_buffer = None;        
    }
}