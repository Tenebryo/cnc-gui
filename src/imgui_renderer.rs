use imgui::{Context, FontConfig, FontSource};
use imgui_winit_support::{HiDpiMode, WinitPlatform};



use vulkano::command_buffer::AutoCommandBufferBuilder;
use vulkano::device::{Device, DeviceExtensions};
use vulkano::image::{ImageUsage, SwapchainImage};
use vulkano::instance::{Instance, PhysicalDevice};
use vulkano::swapchain;
use vulkano::swapchain::{
    AcquireError, ColorSpace, FullscreenExclusive, PresentMode, SurfaceTransform, Swapchain,
    SwapchainCreationError,
};
use vulkano::format::Format;
use vulkano::sync;
use vulkano::device::Queue;
use vulkano::sync::{FlushError, GpuFuture};
use vulkano::swapchain::Surface;


use vulkano_win::VkSurfaceBuild;

use winit::event_loop::{EventLoop};
use winit::window::{Window, WindowBuilder};

use std::sync::Arc;

use imgui_vulkano_renderer::Renderer;


use crate::clipboard;


pub struct System {
    pub device : Arc<Device>,
    pub queue : Arc<Queue>,
    pub surface: Arc<Surface<Window>>,
    pub swapchain : Arc<Swapchain<Window>>,
    pub images : Vec<Arc<SwapchainImage<Window>>>,
    pub imgui: Context,
    pub platform: WinitPlatform,
    pub renderer: Renderer,
    pub font_size: f32,
    pub previous_frame_end : Option<Box<dyn GpuFuture>>,
    pub acquire_future : Option<Box<dyn GpuFuture>>,
    pub recreate_swapchain : bool,
}

pub fn init(title: &str, event_loop : &EventLoop<()>) -> System {


    let required_extensions = vulkano_win::required_extensions();
    let instance = Instance::new(None, &required_extensions, None).unwrap();
    
    let physical = PhysicalDevice::enumerate(&instance).next().unwrap();

    let title = match title.rfind('/') {
        Some(idx) => title.split_at(idx + 1).1,
        None => title,
    };

    let surface = WindowBuilder::new()
        .with_title(title.to_owned())
        .build_vk_surface(&event_loop, instance.clone())
        .expect("Failed to create a window");


    let queue_family = physical
        .queue_families()
        .find(|&q| {
            // We take the first queue that supports drawing to our window.
            q.supports_graphics() && surface.is_supported(q).unwrap_or(false)
        })
        .unwrap();

    let device_ext = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::none()
    };
    let (device, mut queues) = Device::new(
        physical,
        physical.supported_features(),
        &device_ext,
        [(queue_family, 0.5)].iter().cloned(),
    )
    .unwrap();
    
    let queue = queues.next().unwrap();

    // not sure why this was needed
    #[allow(unused_assignments)]
    let mut format = Format::R8G8B8A8Srgb;

    let (swapchain, images) = {
        let caps = surface.capabilities(physical).unwrap();

        let alpha = caps.supported_composite_alpha.iter().next().unwrap();

        format = caps.supported_formats[0].0;

        let dimensions: [u32; 2] = surface.window().inner_size().into();

        let image_usage = ImageUsage {
            transfer_destination : true,
            ..ImageUsage::color_attachment()
        };

        Swapchain::new(
            device.clone(),
            surface.clone(),
            caps.min_image_count,
            format,
            dimensions,
            1,
            image_usage,
            &queue,
            SurfaceTransform::Identity,
            alpha,
            PresentMode::Fifo,
            FullscreenExclusive::Default,
            true,
            ColorSpace::SrgbNonLinear,
        )
        .unwrap()
    };

    let mut imgui = Context::create();
    imgui.set_ini_filename(None);

    if let Some(backend) = clipboard::init() {
        imgui.set_clipboard_backend(Box::new(backend));
    } else {
        eprintln!("Failed to initialize clipboard");
    }

    let mut platform = WinitPlatform::init(&mut imgui);
    platform.attach_window(imgui.io_mut(), &surface.window(), HiDpiMode::Rounded);

    let hidpi_factor = platform.hidpi_factor();
    let font_size = (13.0 * hidpi_factor) as f32;
    imgui.fonts().add_font(&[
        FontSource::DefaultFontData {
            config: Some(FontConfig {
                size_pixels: font_size,
                ..FontConfig::default()
            }),
        },
        // FontSource::TtfData {
        //     data: include_bytes!("../resources/mplus-1p-regular.ttf"),
        //     size_pixels: font_size,
        //     config: Some(FontConfig {
        //         rasterizer_multiply: 1.75,
        //         glyph_ranges: FontGlyphRanges::japanese(),
        //         ..FontConfig::default()
        //     }),
        // },
    ]);

    imgui.io_mut().font_global_scale = (1.0 / hidpi_factor) as f32;

    let renderer = Renderer::init(&mut imgui, device.clone(), queue.clone(), format).expect("Failed to initialize renderer");

    let previous_frame_end = Some(sync::now(device.clone()).boxed());

    System {
        device,
        queue,
        surface,
        swapchain,
        images,
        imgui,
        platform,
        renderer,
        font_size,
        previous_frame_end,
        acquire_future : None,
        recreate_swapchain : false,
    }
}

impl System {
    pub fn start_frame(&mut self) -> Result<(AutoCommandBufferBuilder, Arc<SwapchainImage<Window>>, usize),()> {

            
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        if self.recreate_swapchain {
            let dimensions: [u32; 2] = self.surface.window().inner_size().into();
            let (new_swapchain, new_images) =
                match self.swapchain.recreate_with_dimensions(dimensions) {
                    Ok(r) => r,
                    Err(SwapchainCreationError::UnsupportedDimensions) => return Err(()),
                    Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
                };

            self.images = new_images;
            self.swapchain = new_swapchain;
            self.recreate_swapchain = false;
        }

            
        let (image_num, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None) {
                Ok(r) => r,
                Err(AcquireError::OutOfDate) => {
                    self.recreate_swapchain = true;
                    return Err(());
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        if suboptimal {
            self.recreate_swapchain = true;
        }

        self.acquire_future = Some(Box::new(acquire_future));


        let cmd_buf_builder = AutoCommandBufferBuilder::new(self.device.clone(), self.queue.family())
            .expect("Failed to create command buffer");


        Ok((cmd_buf_builder, self.images[image_num].clone(), image_num))
    }

    pub fn end_frame(&mut self, cmd_buf_builder : AutoCommandBufferBuilder, image_num : usize) {

        let cmd_buf = cmd_buf_builder.build()
            .expect("Failed to build command buffer");

        let future = self.previous_frame_end
            .take()
            .unwrap()
            .join(core::mem::replace(&mut self.acquire_future, None).expect("No acquire future, was `start_frame` called?"))
            .then_execute(self.queue.clone(), cmd_buf)
            .unwrap()
            .then_signal_fence()
            .then_swapchain_present(self.queue.clone(), self.swapchain.clone(), image_num);

        match future.flush() {
            Ok(_) => {
                // self.previous_frame_end = Some(future.boxed());
                self.previous_frame_end = Some(Box::new(future));
            }
            Err(FlushError::OutOfDate) => {
                self.recreate_swapchain = true;
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }
}
