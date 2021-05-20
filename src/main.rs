#![allow(dead_code)]
#![feature(drain_filter)]

#[macro_use]
extern crate pest_derive;
#[macro_use]
extern crate gfx;

use std::{path::PathBuf, sync::{Arc, atomic::{AtomicBool, Ordering}}, time::Instant};

use cgmath::{Deg, InnerSpace, Matrix4, Quaternion, Rad, Rotation3, Rotation, Vector2, Vector3};
use gfx::{Factory, traits::FactoryExt};
use grbl::GRBLConnection;

use imgui::ImString;
use rand::{thread_rng, Rng};

use crate::simulation::GcodeProgram;

macro_rules! im_strf {
    ($($args:tt)*) => {
        &imgui::ImString::from(format!($($args)*))
    };
}

mod grbl;
mod simulation;
mod imgui_renderer;
mod util;
mod visuals;
mod gcode;

struct WindowRect {
    pos : [f32; 2],
    size : [f32; 2],
}

struct UiData {

}

fn main() {

    let async_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();
    
    let mut system = imgui_renderer::init("GRBL Driver");

    let mut rng = thread_rng();

    let mut ports = serialport::available_ports().expect("No ports found!");

    // let mut connection : Option<(usize, usize)> = None;
    let mut connection : Arc<Option<(usize, GRBLConnection)>> = Arc::new(None);

    let mut target_point = [0.0; 3];
    let mut realtime_active = false;

    let mut jog_absolute_machine_coords = false;


    let dialog_open = Arc::new(AtomicBool::new(false));
    let open_path = Arc::new(std::sync::Mutex::new(imgui::ImString::new(String::new())));

    
    let mut baud_rate_i = 7usize;
    let mut baud_rate = 115_200i32;

    let mut tex = None;
    let mut depth = None;

    let mut machine_coords = [0.0; 3];
    let mut work_coords = [0.0; 3];

    let mut spindle_rpm_setpoint = 7200;
    let mut spindle_on = false;


    let mut gcode_programs = Arc::new(std::sync::Mutex::new(vec![]));
    let mut active_program : Option<GcodeProgram> = None;

    let mut line_renderer = visuals::LineRenderer::new(&mut system.render_sys.factory);
    let mut viewport_needs_update = false;

    let mut vertex_buffer = None;


    let mut line_data = std::iter::repeat_with(|| [
        rng.gen_range(0.0..=1.00),
        rng.gen_range(0.0..=1.00),
        rng.gen_range(0.0..=1.00)
    ]).take(10_000);


    let main_loop_start = Instant::now();

    let mut center = Vector3::new(0.0, 0.0, 0.0);
    let mut scale = 0.05;
    let mut orientation : Quaternion<f32> = Quaternion::from_arc(Vector3::unit_x(), Vector3::unit_x(), None);

    let mut axis_grid_lines = visuals::line_grid(0.0, 0.0, 50.0, 50.0, 7, 7, [0.5, 0.5, 0.5], None);

    axis_grid_lines.extend_from_slice(&[
        // x axis
        visuals::Vertex {
            pos: [0.0; 3],
            color : [1.0, 0.0, 0.0],
            time : 0.0
        },
        visuals::Vertex {
            pos: [350.0, 0.0, 0.0],
            color : [1.0, 0.0, 0.0],
            time : 0.0
        },
        // x axis
        visuals::Vertex {
            pos: [0.0; 3],
            color : [0.0, 1.0, 0.0],
            time : 0.0
        },
        visuals::Vertex {
            pos: [0.0, 350.0, 0.0],
            color : [0.0, 1.0, 0.0],
            time : 0.0
        },
        // x axis
        visuals::Vertex {
            pos: [0.0; 3],
            color : [0.0, 0.0, 1.0],
            time : 0.0
        },
        visuals::Vertex {
            pos: [0.0, 0.0, 350.0],
            color : [0.0, 0.0, 1.0],
            time : 0.0
        }
    ]);

    let axis_grid_vb = system.render_sys.factory.create_vertex_buffer_with_slice(&axis_grid_lines, ());

    system.main_loop(move |_, ui, sys| {

        use imgui::*;
        use imgui::im_str;

        let wdth = sys.window().inner_size().width as f32;
        let hght = sys.window().inner_size().height as f32;

        if let Some(tok) = ui.begin_main_menu_bar() {
            
            if let Some(tok) = ui.begin_menu(im_str!("File"), true) {

                if MenuItem::new(im_str!("Open")).build(ui) {

                    if !dialog_open.fetch_or(true, Ordering::SeqCst) {

                        let dialog_open = dialog_open.clone();
                        async_runtime.spawn_blocking(move || {

                            match nfd::open_pick_folder(None) {
                                Ok(nfd::Response::Okay(_))     => {
                                },
                                Ok(nfd::Response::Cancel)              => println!("User canceled"),
                                _ => {}
                            }

                            dialog_open.store(false, Ordering::SeqCst);
                        });
                    }
                }

                tok.end(ui);
            }
            tok.end(ui);
        }

        let menu_bar_h = 19.0;

        let side_panel_w = 300.0;

        let port_window_h = 256.0;

        let port_window_rect = WindowRect{
            pos : [0.0, menu_bar_h],
            size : [side_panel_w, port_window_h],
        };

        let gcode_program_window_rect = WindowRect{
            pos : [0.0, menu_bar_h + port_window_h],
            size : [side_panel_w, hght - menu_bar_h - port_window_h],
        };

        let machine_window_rect = WindowRect{
            pos : [wdth - side_panel_w, menu_bar_h],
            size : [side_panel_w, hght - menu_bar_h],
        };

        let viewport_window_rect = WindowRect{
            pos : [port_window_rect.size[0], menu_bar_h],
            size : [wdth - port_window_rect.size[0] - machine_window_rect.size[0], hght - menu_bar_h],
        };

        // this window is used to connect and disconnect from serial
        // ports and to select the baud rate for communication
        imgui::Window::new(im_str!("Port List"))
            .position(port_window_rect.pos, imgui::Condition::Always)
            .size(port_window_rect.size, imgui::Condition::Always)
            .scroll_bar(true)
            .collapsible(false)
            .resizable(false)
            .build(ui, || {

                let [ww, wh] = ui.window_content_region_max();


                let baud_rates = [1200, 2400, 4800, 9600, 19200, 38400, 57600, 115200];
                let baud_rates_str = [
                    im_str!("1200"), 
                    im_str!("2400"), 
                    im_str!("4800"), 
                    im_str!("9600"), 
                    im_str!("19200"), 
                    im_str!("38400"), 
                    im_str!("57600"), 
                    im_str!("115200"), 
                    im_str!("Other")
                ];

                ComboBox::new(im_str!("Baud Rate"))
                    .build_simple_string(ui, &mut baud_rate_i, &baud_rates_str);

                if baud_rate_i == baud_rates.len() {
                    ui.input_int(im_str!("Other##baud_rate_input"), &mut baud_rate)
                        .build();
                } else {
                    baud_rate = baud_rates[baud_rate_i];
                }

                ui.separator();

                if let Some((j, _)) = *connection {
                    for (i, p) in ports.iter().enumerate() {
                        ui.text(&format!("[{:2}] {:?}", i, p.port_name));
                        if i == j {
                            ui.same_line(ww - 80.0);
                            if ui.small_button(im_strf!("Disconnect##{}", p.port_name)) {
                                println!("Disconnected {}", p.port_name);
                                connection = Arc::new(None);
                            }
                        }
                    }
                } else {
                    for (i, p) in ports.iter().enumerate() {
                        ui.text(&format!("[{:2}] {:?}", i, p.port_name));
                        ui.same_line(ww - 80.0);
                        if ui.small_button(im_strf!("Connect##{}", p.port_name)) {
                            println!("Connected {}", p.port_name);
                            connection = Arc::new(Some((i, GRBLConnection::open(&p.port_name).unwrap())));
                        }
                    }
                }

                ui.separator();

                if ui.button(im_str!("Refresh"), [80.0, 20.0]) {
                    ports = serialport::available_ports().expect("No ports found!");
                    println!("refreshed.");
                }
            });

        // this window shows a list of loaded gcode programs, ui to load them, and ui to select and run programs
        imgui::Window::new(im_str!("Programs"))
            .position(gcode_program_window_rect.pos, imgui::Condition::Always)
            .size(gcode_program_window_rect.size, imgui::Condition::Always)
            .scroll_bar(true)
            .collapsible(false)
            .resizable(false)
            .build(ui, || {
                
                if ui.button(im_str!("Import"), [ui.window_content_region_width(), 24.0]) {
                    
                    if !dialog_open.fetch_or(true, Ordering::SeqCst) {
                        let dialog_open = dialog_open.clone();
                        let gcode_programs = gcode_programs.clone();
                        async_runtime.spawn_blocking(move || {

                            match nfd::open_file_multiple_dialog(None, None) {
                                Ok(nfd::Response::Okay(file_path))     => {
                                    println!("File path = {:?}", file_path);
                                    // *gcode_programs.lock().unwrap() = file_path.into();
                                },
                                Ok(nfd::Response::OkayMultiple(paths)) => {
                                    for path in paths {
                                        let program = std::fs::read_to_string(&path).unwrap();
                                        let gcode_program = GcodeProgram::load(PathBuf::from(&path), program);
                                        gcode_programs.lock().unwrap().push(gcode_program);
                                    }
                                }
                                Ok(nfd::Response::Cancel)              => println!("User canceled"),
                                _ => {println!("test...");}
                            }

                            dialog_open.store(false, Ordering::SeqCst);
                        });
                    }
                }

                ui.separator();
                

                let active_path = active_program.as_ref().map(|p| p.filepath.clone());

                gcode_programs.lock().unwrap().drain_filter(|program| {

                    if Some(program.filepath.clone()) == active_path {
                        ui.text(format!("[{:?}]", program.filepath.file_name().unwrap()));
                    } else {
                        ui.text(format!(" {:?} ", program.filepath.file_name().unwrap()));
                    }

                    ui.same_line(ui.window_content_region_width() - 64.0);

                    let load_id = ImString::from(format!("Load##{:?}", program.filepath));
                    let del_id = ImString::from(format!("X##{:?}", program.filepath));

                    if ui.small_button(&load_id) {
                        active_program = Some(program.clone());
                        
                        let verts = program.motionpath.iter()
                            .map(|mp| {
                                visuals::Vertex {
                                    pos : mp.pos.into(),
                                    color : if mp.ty == simulation::MotionType::Linear {[1.0, 0.0, 0.0]} else {[0.0, 0.0, 1.0]},
                                    time : mp.time,
                                }
                            })
                            .collect::<Vec<_>>();

                        vertex_buffer = Some(sys.factory.create_vertex_buffer_with_slice(&verts, ()));
                        viewport_needs_update = true;
                    }

                    ui.same_line(ui.window_content_region_width() - 16.0);
                    
                    if ui.small_button(&del_id) {
                        return true;
                    }

                    false
                });
            });

        // this window contains controls used to give realtime commands
        // to GRBL, such as feed hold, cycle start, abort, and jog.
        imgui::Window::new(im_str!("Realtime Control"))
            .position(machine_window_rect.pos, imgui::Condition::Always)
            .size(machine_window_rect.size, imgui::Condition::Always)
            .scroll_bar(true)
            .collapsible(false)
            .resizable(false)
            .build(ui, || {


                let [ww, wh] = ui.window_content_region_max();

                ui.text("Tool Position");
                ui.separator();

                ui.input_float3(im_str!("Machine Coords"), &mut machine_coords)
                    .build();
                ui.input_float3(im_str!("Work Coords"), &mut machine_coords)
                    .build();


                ui.separator();
                ui.text(if spindle_on {"Spindle (on)"} else {"Spindle (off)"});
                ui.separator();

                Slider::new(im_str!("Target RPM"))
                    .range(7200..=24000)
                    .build(ui, &mut spindle_rpm_setpoint);

                spindle_rpm_setpoint = (spindle_rpm_setpoint as f32 / 100.0).round() as i32 * 100;

                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        if spindle_on {[0.0, 0.1, 0.0, 1.0]} else {[0.0, 0.5, 0.0, 1.0]}),
                    (StyleColor::ButtonActive,  if spindle_on {[0.0, 0.1, 0.0, 1.0]} else {[0.0, 0.75, 0.0, 1.0]}),
                    (StyleColor::ButtonHovered, if spindle_on {[0.0, 0.1, 0.0, 1.0]} else {[0.0, 1.0, 0.0, 1.0]})
                ]);
                spindle_on |= ui.button(im_str!("On"), [ww / 2.0 - 10.0, 24.0]);
                tok.pop(ui);

                ui.same_line(ww / 2.0 + 10.0);

                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        if spindle_on {[0.5, 0.0, 0.0, 1.0]} else {[0.1, 0.0, 0.0, 1.0]}),
                    (StyleColor::ButtonActive,  if spindle_on {[0.75, 0.0, 0.0, 1.0]} else {[0.1, 0.0, 0.0, 1.0]}),
                    (StyleColor::ButtonHovered, if spindle_on {[1.0, 0.0, 0.0, 1.0]} else {[0.1, 0.0, 0.0, 1.0]})
                ]);
                spindle_on &= !ui.button(im_str!("Off"), [ww / 2.0 - 10.0, 24.0]);
                tok.pop(ui);    

                ui.separator();

                ui.separator();
                ui.text("Jogging");
                ui.separator();

                ui.separator();
                ui.text("Move to Point");
                ui.separator();

                imgui::InputFloat3::new(ui, im_str!("Position"), &mut target_point)
                    .build();


                    
                ui.checkbox(im_str!("Machine Basis"), &mut jog_absolute_machine_coords);

                if ui.button(im_str!("Go"), [ww * 0.75, 20.0]) {
                    println!("Jog to {:?}", target_point);
                }

                
                ui.checkbox(im_str!("Realtime Controls Active"), &mut realtime_active);
            });


        let tok = ui.push_style_var(StyleVar::WindowPadding([0.0; 2]));

        // This window shows a render of the toolpath and (TODO) a representation of the machine.
        imgui::Window::new(im_str!("Viewport"))
            .position(viewport_window_rect.pos, imgui::Condition::Always)
            .size(viewport_window_rect.size, imgui::Condition::Always)
            .scroll_bar(false)
            .scrollable(false)
            .collapsible(false)
            .resizable(false)
            .build(ui, || {

                let dim = ui.window_content_region_max();

                if ui.is_window_hovered() {
                    let io = ui.io();
                    scale *= 1.1f32.powf(io.mouse_wheel);
                    
                    if io.mouse_wheel.abs() < 0.1 {
                        viewport_needs_update = true;
                    }
                    let mouse_delta = io.mouse_delta;
                    if io.mouse_down[0] {
                        center += (1.0 / (scale * dim[0])) * (orientation.invert() * Vector3::new(mouse_delta[0], mouse_delta[1], 0.0));

                        if Vector2::from(mouse_delta).magnitude() < 1.0 {
                            viewport_needs_update = true;
                        }
                    }
                    if io.mouse_down[1] {
                        let axis = Vector3::new(-mouse_delta[1], mouse_delta[0], 0.0);
                        let angle = axis.magnitude() * 0.001;
                        orientation = (Quaternion::from_axis_angle(axis, Rad(angle)) * orientation).normalize();
                        if Vector2::from(mouse_delta).magnitude() < 1.0 {
                            viewport_needs_update = true;
                        }
                    }
                }

                let nw = dim[0] as u16;
                let nh = dim[1] as u16;

                let tex = tex.get_or_insert_with(|| {
                    sys.create_texture(nw, nh)
                });

                if let gfx::texture::Kind::D2(w, h, _) = tex.1.get_info().kind {
                    if w != nw || h != nh {
                        // recreate texture
                        println!("Recreating Render Texture");
                        *tex = sys.recreate_texture(tex.0, nw, nh);
                        depth = Some(sys.factory.create_depth_stencil(nw, nh).unwrap().2);
                        viewport_needs_update = true;
                    }
                }

                let cam_tr = 
                    Matrix4::from_nonuniform_scale(1.0, dim[0] / dim[1], -0.001) * 
                    Matrix4::from(orientation) * 
                    Matrix4::from_scale(scale) * 
                    Matrix4::from_translation(center);

                let depth = depth.get_or_insert_with(|| {
                    println!("Recreating Depth Texture");
                    sys.factory.create_depth_stencil(nw, nh).unwrap().2
                });

                if viewport_needs_update {
                    let mut encoder: gfx::Encoder<_, _> = sys.factory.create_command_buffer().into();
                    encoder.clear_depth(&depth, 1.0);
                    encoder.clear(&tex.2, [0x64 as f32 / 256.0, 0x95 as f32 / 256.0, 0xED as f32 / 256.0, 1.0]);
                    if let Some(_) = &active_program {
                        if let Some(vertex_buffer) = vertex_buffer.clone() {
                            //mains_loop_start.elapsed().as_secs_f32()/8.0
                            line_renderer.draw_line_strip(&mut encoder, &tex.2, depth, cam_tr, vertex_buffer);
                        }
                    }

                    line_renderer.draw_line_list(&mut encoder, &tex.2, depth, cam_tr * Matrix4::from_translation(work_coords.into()), axis_grid_vb.clone());
                    viewport_needs_update = false;
                    encoder.flush(&mut sys.device);
                }
                
                Image::new(tex.0, dim)
                    .build(ui);
            });

        tok.pop(ui);
        
    });
}