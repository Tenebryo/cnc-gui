use std::sync::Arc;
use imgui::TextureId;
use vulkano::{format::Format, image::AttachmentImage, render_pass::{Framebuffer, FramebufferAbstract, RenderPass}};
use vulkano::sampler::Sampler;
use crate::imgui_renderer::System;
use vulkano::image::view::ImageView;
use vulkano::image::{ImageCreateFlags, ImageDimensions, ImageUsage, StorageImage};



pub struct Viewport {
    pub image : Option<Arc<StorageImage>>,
    pub depth_image : Option<Arc<AttachmentImage>>,
    pub msaa_image : Option<Arc<AttachmentImage>>,
    pub texture_id : Option<TextureId>,
    pub width : u32,
    pub height : u32,
}

impl Viewport {

    pub fn new() -> Self {

        Viewport {
            width : 1,
            height : 1,
            image : None,
            depth_image : None,
            msaa_image : None,
            texture_id : None,
        }
    }

    pub fn update(&mut self, system : &mut System, width : u32, height : u32) {
        if self.width != width || self.height != height {

            self.width = width;
            self.height = height;

            let image =
                StorageImage::with_usage(
                    system.device.clone(), 
                    ImageDimensions::Dim2d{width, height, array_layers:1},
                    Format::R8G8B8A8Unorm, 
                    ImageUsage{
                        sampled : true,
                        ..ImageUsage::color_attachment()
                    }, 
                    ImageCreateFlags::default(),
                    vec![system.queue.family()]
                ).expect("Failed to create viewport storage image");

            if self.texture_id == None {

                let texture_id = system.renderer.textures().insert((ImageView::new(image.clone()).unwrap(), Sampler::simple_repeat_linear(system.device.clone())));
                self.texture_id = Some(texture_id);
            } else {

                system.renderer.textures().replace(self.texture_id.unwrap(), (ImageView::new(image.clone()).unwrap(), Sampler::simple_repeat_linear(system.device.clone())));
            }


            let depth_buffer = AttachmentImage::transient_multisampled_input_attachment(
                system.device.clone(), 
                [width, height],
                4,
                Format::D16Unorm
            ).unwrap();


            let msaa_buffer = AttachmentImage::transient_multisampled_input_attachment(
                system.device.clone(), 
                [width, height],
                4,
                Format::R8G8B8A8Unorm
            ).unwrap();

            self.image = Some(image);
            self.depth_image = Some(depth_buffer);
            self.msaa_image = Some(msaa_buffer);

            println!("recreated viewport buffer")
        };
    }

    pub fn create_framebuffer(&self, render_pass : Arc<RenderPass>) -> Option<Arc<dyn FramebufferAbstract + Send + Sync>> {

        if let (Some(ref image), Some(ref depth_buffer), Some(ref msaa_buffer)) = (&self.image, &self.depth_image, &self.msaa_image) {

            let framebuffer = Arc::new(
                Framebuffer::start(render_pass)
                    .add(ImageView::new(depth_buffer.clone()).unwrap()).unwrap()
                    .add(ImageView::new(msaa_buffer.clone()).unwrap()).unwrap()
                    .add(ImageView::new(image.clone()).unwrap()).unwrap()
                    .build().unwrap()
            );

            Some(Arc::new(framebuffer))
        } else {
            None
        }
    }
}