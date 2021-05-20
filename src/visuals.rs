use gfx::{Factory, PipelineState, ProgramInfo, Resources, Slice, format::{BlendFormat, D24_S8}, gfx_defines, handle::{Buffer, DepthStencilView, RenderTargetView}, preset::blend, pso::{Descriptor, InitError, PipelineInit}, state::{ColorMask, RasterMethod, Rasterizer}, traits::FactoryExt};


use gfx;

// pub type ColorFormat = gfx::format::Srgba8;
pub type ColorFormat = [f32;4];
// pub type ColorFormat = gfx::format::Rgba32F;

gfx_defines! {
    vertex Vertex {
        pos: [f32; 3] = "pos",
        color: [f32; 3] = "col",
        time: f32 = "time",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        transform: gfx::Global<[[f32; 4]; 4]> = "matrix",
        time_interval : gfx::Global<f32> = "time_interval",
        tint : gfx::Global<[f32; 4]> = "tint",
        out_color: gfx::RenderTarget<ColorFormat> = "Target0",
        out_depth: gfx::DepthTarget<gfx::format::Depth32F> =
            gfx::state::Depth{
                fun : gfx::state::Comparison::LessEqual,
                write : true,
            },
    }
}

pub struct LineRenderer {
    pipeline_strip1 : PipelineState<gfx_device_gl::Resources, pipe::Meta>,
    pipeline_strip5 : PipelineState<gfx_device_gl::Resources, pipe::Meta>,
    pipeline_list1 : PipelineState<gfx_device_gl::Resources, pipe::Meta>,
    pipeline_list5 : PipelineState<gfx_device_gl::Resources, pipe::Meta>,
}


use cgmath::{Matrix4, Vector3};
use gfx_device_gl::CommandBuffer;

impl LineRenderer {

    
    pub fn new<F: gfx::Factory<gfx_device_gl::Resources>>(factory : &mut F) -> LineRenderer {
        

        const VS_SRC : &[u8] = include_bytes!("shaders/line.vert");
        const FS_SRC : &[u8] = include_bytes!("shaders/line.frag");

        let program = factory.link_program(VS_SRC, FS_SRC).unwrap();

        let pipeline_strip1 = factory.create_pipeline_from_program(&program, gfx::Primitive::LineStrip, Rasterizer{method : RasterMethod::Line(1), ..Rasterizer::new_fill()}, pipe::new()).unwrap();
        let pipeline_strip5 = factory.create_pipeline_from_program(&program, gfx::Primitive::LineStrip, Rasterizer{method : RasterMethod::Line(5), ..Rasterizer::new_fill()}, pipe::new()).unwrap();

        let pipeline_list1 = factory.create_pipeline_from_program(&program, gfx::Primitive::LineList, Rasterizer{method : RasterMethod::Line(1), ..Rasterizer::new_fill()}, pipe::new()).unwrap();
        let pipeline_list5 = factory.create_pipeline_from_program(&program, gfx::Primitive::LineList, Rasterizer{method : RasterMethod::Line(5), ..Rasterizer::new_fill()}, pipe::new()).unwrap();
        

        // let pipeline = factory.create_pipeline_simple(VS_SRC, FS_SRC, pipe::new()).unwrap();

        LineRenderer {
            pipeline_strip1,
            pipeline_strip5,
            pipeline_list1,
            pipeline_list5,
        }
    }

    pub fn draw_line_strip(
        &self,
        encoder : &mut gfx::Encoder<gfx_device_gl::Resources, CommandBuffer>, 
        target : &RenderTargetView<gfx_device_gl::Resources, [f32; 4]>, 
        depth : &DepthStencilView<gfx_device_gl::Resources, gfx::format::Depth32F>, 
        camera_transform : Matrix4<f32>,
        (vb, slice) : (Buffer<gfx_device_gl::Resources, Vertex>, Slice<gfx_device_gl::Resources>),
    ) {

        encoder.clear(target, [0x64 as f32 / 256.0, 0x95 as f32 / 256.0, 0xED as f32 / 256.0, 1.0]);
        
        encoder.clear_depth(&depth, 1.0);

        let mut data = pipe::Data {
            vbuf : vb,
            transform : camera_transform.into(),
            time_interval : 0.1,
            tint : [0.25; 4],
            out_color : target.clone(),
            out_depth : depth.clone(),
        };

        encoder.draw(&slice, &self.pipeline_strip5, &data);

        data.tint = [1.0; 4];

        encoder.draw(&slice, &self.pipeline_strip1, &data);
    }

    pub fn draw_line_list(
        &self,
        encoder : &mut gfx::Encoder<gfx_device_gl::Resources, CommandBuffer>, 
        target : &RenderTargetView<gfx_device_gl::Resources, [f32; 4]>, 
        depth : &DepthStencilView<gfx_device_gl::Resources, gfx::format::Depth32F>, 
        camera_transform : Matrix4<f32>,
        (vb, slice) : (Buffer<gfx_device_gl::Resources, Vertex>, Slice<gfx_device_gl::Resources>),
    ) {
        
        let mut data = pipe::Data {
            vbuf : vb,
            transform : camera_transform.into(),
            time_interval : 0.1,
            tint : [0.25; 4],
            out_color : target.clone(),
            out_depth : depth.clone(),
        };

        encoder.draw(&slice, &self.pipeline_list5, &data);

        data.tint = [1.0; 4];

        encoder.draw(&slice, &self.pipeline_list1, &data);

    }

}

pub fn clear_render_target(
    sys : &mut crate::imgui_renderer::RenderSystem, 
    target : &RenderTargetView<gfx_device_gl::Resources, [f32; 4]>, 
) {
    
    let mut encoder: gfx::Encoder<_, _> = sys.factory.create_command_buffer().into();

    encoder.clear(target, [0x64 as f32 / 256.0, 0x95 as f32 / 256.0, 0xED as f32 / 256.0, 1.0]);

    encoder.flush(&mut sys.device);
}

pub fn line_grid(x0 : f32, y0 : f32, dx : f32, dy : f32, nx : usize, ny : usize, color : [f32; 3], transform : Option<Matrix4<f32>>) -> Vec<Vertex> {

    fn deref_v(v : &Vertex) -> Vertex {*v}

    let mut points = Vec::with_capacity(2 * (nx + ny));

    for i in 0..nx {
        let i = i as f32;
        points.extend_from_slice(&[
            Vertex {
                pos : [x0 + dx * i, y0, 0.0],
                color,
                time: 0.0,
            },
            Vertex {
                pos : [x0 + dx * i, y0 + dy * (ny as f32), 0.0],
                color,
                time: 0.0,
            }
        ]);
    }
    for i in 0..nx {
        let i = i as f32;
        points.extend_from_slice(&[
            Vertex {
                pos : [x0, y0 + dy * i, 0.0],
                color,
                time: 0.0,
            },
            Vertex {
                pos : [x0 + dx * (nx as f32), y0 + dy * i, 0.0],
                color,
                time: 0.0,
            }
        ]);
    }

    if let Some(transform) = transform {
        for p in points.iter_mut() {
            p.pos = (transform * Vector3::from(p.pos).extend(1.0)).truncate().into();
        }
    }

    points
}