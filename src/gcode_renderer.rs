use imgui::TextureId;
use imgui_vulkano_renderer::Renderer;
use vulkano::{buffer::{BufferUsage, CpuAccessibleBuffer, CpuBufferPool}, command_buffer::DynamicState, pipeline::{input_assembly::PrimitiveTopology, viewport::Viewport}, sampler::Sampler, sync::{FlushError, GpuFuture}};
use vulkano::image::AttachmentImage;
use vulkano::framebuffer::Framebuffer;
use vulkano::{command_buffer::{AutoCommandBufferBuilder, SubpassContents}, image::{Dimensions, ImageUsage}};
use vulkano::{image::StorageImage, pipeline::GraphicsPipeline};
use vulkano::framebuffer::Subpass;
use std::sync::Arc;

use vulkano::{impl_vertex, pipeline::GraphicsPipelineAbstract};

use vulkano::framebuffer::RenderPassAbstract;

use vulkano::format::Format;

pub mod line_fs {vulkano_shaders::shader!{ty: "fragment",path: "src/shaders/line.frag",               include: [],}}
pub mod line_vs {vulkano_shaders::shader!{ty: "vertex",  path: "src/shaders/line.vert",               include: [],}}

use crate::imgui_renderer::{System};

#[derive(Debug, Default)]
pub struct Vertex{
    pub pos : [f32;3],
    pub col : [f32;4],
    pub time : f32,
}

impl_vertex!(Vertex, pos, col, time);

pub struct GCodeRenderer {
    pub pipeline : Arc<dyn GraphicsPipelineAbstract + Send + Sync>,
    pub render_pass : Arc<dyn RenderPassAbstract + Send + Sync>,
    pub image : Option<Arc<StorageImage<Format>>>,
    pub vertex_pool : CpuBufferPool<Vertex>,
    pub vertex_buffer : Option<Arc<CpuAccessibleBuffer<[Vertex]>>>,
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
                        samples: 1,
                    },
                    color: {
                        load: Clear,
                        store: Store,
                        format: Format::R8G8B8A8Unorm,
                        samples: 1,
                    }
                },
                passes: [
                    {
                        color: [color],
                        depth_stencil: {depth},
                        input: []
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

    pub fn render(&mut self, system : &System, renderer : &mut Renderer, width : u32, height : u32, cmd_buf_builder : &mut AutoCommandBufferBuilder) {

        if self.image.as_ref().map(|i| i.dimensions() != Dimensions::Dim2d{width,height}).unwrap_or(true) {
            let image =
                StorageImage::with_usage(
                    system.device.clone(), 
                    vulkano::image::Dimensions::Dim2d{width, height}, 
                    Format::R8G8B8A8Unorm, 
                    ImageUsage{
                        sampled : true,
                        ..ImageUsage::color_attachment()
                    }, 
                    vec![system.queue.family()]
                ).expect("Failed to create viewport storage image");

            if self.texture_id == None {

                let texture_id = renderer.textures().insert((image.clone(), Sampler::simple_repeat_linear(system.device.clone())));
                self.texture_id = Some(texture_id);
            } else {

                renderer.textures().replace(self.texture_id.unwrap(), (image.clone(), Sampler::simple_repeat_linear(system.device.clone())));
            }

            self.image = Some(image);

            println!("recreated viewport buffer")
        };

        if let Some(ref mut image) = self.image {

            let depth_buffer = AttachmentImage::transient_input_attachment(system.device.clone(), [width, height], Format::D16Unorm).unwrap();

            let framebuffer = Arc::new(
                Framebuffer::start(self.render_pass.clone())
                    .add(depth_buffer.clone()).unwrap()
                    .add(image.clone()).unwrap()
                    .build().unwrap()
            );

            cmd_buf_builder.begin_render_pass(
                framebuffer, 
                SubpassContents::Inline, 
                vec![1.0.into(), [0.0, 0.0, 0.0, 1.0].into()]
            ).expect("failed to start render pass");


            let ds = DynamicState {
                viewports : Some(vec![Viewport {
                    origin : [0.0; 2],
                    dimensions : [width as f32, height as f32],
                    depth_range : 0.0..1.0,
                }]),
                ..DynamicState::none()
            };

            if let Some(ref vb) = self.vertex_buffer {

                cmd_buf_builder.draw(self.pipeline.clone(), &ds, vec![vb.clone()], (), line_vs::ty::PushConstants {
                    matrix : Default::default(),
                })
                    .expect("failed to draw line");
            }

            cmd_buf_builder.end_render_pass()
                .expect("Failed to finish render pass");

        }
    }
}