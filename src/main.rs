#![allow(dead_code)]
#![feature(drain_filter)]

#[macro_use]
extern crate pest_derive;

use ui::UIState;

macro_rules! im_strf {
    ($($args:tt)*) => {
        &imgui::ImString::from(format!($($args)*))
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

struct WindowRect {
    pos : [f32; 2],
    size : [f32; 2],
}


fn main() {

    let mut async_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    
    let mut system = imgui_renderer::init("GRBL Driver");

    let mut line_renderer = gcode_renderer::GCodeRenderer::init(&system);


    let mut ui_state = UIState::init();

    system.main_loop(move |system, renderer, _, ui, win| {
        ui_state.frame(ui, &mut async_runtime, win);
    });
}