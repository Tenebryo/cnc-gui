#![allow(dead_code)]
#![feature(drain_filter, array_windows)]

#[macro_use]
extern crate pest_derive;

use vulkano::image::view::ImageView;
use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use ui::UIState;
use winit::event_loop::EventLoop;

macro_rules! im_strf {
    ($($args:tt)*) => {
        &imgui::ImString::from(format!($($args)*))
    };
}

macro_rules! im_stringf {
    ($($args:tt)*) => {
        imgui::ImString::from(format!($($args)*))
    };
}

mod grbl;
mod simulation;
mod imgui_renderer;
mod util;
mod viewport;
mod gcode;
mod gcode_renderer;
mod ui;
mod clipboard;
mod rendering;

struct WindowRect {
    pos : [f32; 2],
    size : [f32; 2],
}


fn main() {

    let mut async_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();


    let event_loop = EventLoop::new();
    
    let mut system = imgui_renderer::init("GRBL Driver", &event_loop);

    let mut line_renderer = gcode_renderer::GCodeRenderer::init(&system);


    let mut ui_state = UIState::init();

    let mut viewport = viewport::Viewport::new();


    event_loop.run(move |event, _, control_flow| {

        if ui_state.connection.is_some() {
        }

        match event {
            Event::NewEvents(_) => {
                // imgui.io_mut().update_delta_time(Instant::now());
            }
            Event::MainEventsCleared => {
                system.platform
                    .prepare_frame(system.imgui.io_mut(), &system.surface.window())
                    .expect("Failed to prepare frame");
                system.surface.window().request_redraw();
            }
            Event::RedrawRequested(_) => {

                if let Ok((mut cmd_buf_builder, swapchain_image, image_num)) = system.start_frame() {


                    let mut ui = system.imgui.frame();

                    let run = true;

                    ui_state.frame(&mut ui, &mut async_runtime, &viewport, &mut line_renderer, system.surface.window());


                    if !run {
                        *control_flow = ControlFlow::Exit;
                    }
                    
                    system.platform.prepare_render(&ui, system.surface.window());
                    let draw_data = ui.render();


                    cmd_buf_builder.clear_color_image(swapchain_image.clone(), [0.0; 4].into())
                        .expect("Failed to create image clear command");

                    system.renderer
                        .draw_commands(&mut cmd_buf_builder, system.queue.clone(), ImageView::new(swapchain_image.clone()).unwrap(), draw_data)
                        .expect("Rendering failed");

                    viewport.update(&mut system, ui_state.viewport_dims[0] as u32, ui_state.viewport_dims[1] as u32);

                    if ui_state.viewport_needs_update {
                        line_renderer.render(
                            &mut system,
                            &viewport,
                            &mut cmd_buf_builder,
                            ui_state.tmatrix,
                            ui_state.viewport_dims[0] as u32, ui_state.viewport_dims[1] as u32
                        );
                        ui_state.viewport_needs_update = false;
                    }

                    system.end_frame(cmd_buf_builder, image_num);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            event => {
                system.platform.handle_event(system.imgui.io_mut(), system.surface.window(), &event);
            }
        }
    });
}