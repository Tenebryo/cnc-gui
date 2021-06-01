
use cgmath::*;
use std::time::Instant;
use crate::{WindowRect, gcode_renderer::GCodeRenderer};
use std::sync::Arc;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use cgmath::{InnerSpace, Quaternion, Rad, Rotation3, Rotation, Vector2, Vector3};
use serialport::SerialPortInfo;
use crate::grbl::GRBLConnection;

use std::sync::atomic::AtomicBool;

use winit::window::Window;

use crate::simulation::GcodeProgram;

pub struct UIState {
    pub ports                       : Vec<SerialPortInfo>,
    pub connection                  : Option<(usize, GRBLConnection)>,
    pub dialog_open                 : Arc<AtomicBool>,
    pub open_path                   : Arc<std::sync::Mutex<imgui::ImString>>,
    pub baud_rate_i                 : usize,
    pub baud_rate                   : i32,
    pub spindle_rpm_setpoint        : i32,
    pub spindle_on                  : bool,
    pub gcode_programs              : Arc<std::sync::Mutex<Vec<GcodeProgram>>>,
    pub active_program              : Option<GcodeProgram>,
    pub machine_coords              : [f32; 3],
    pub work_coords                 : [f32; 3],
    pub main_loop_start             : Instant,
    pub scale                       : f32,
    pub center                      : Vector3<f32>,
    pub orientation                 : Quaternion<f32>,
    pub tmatrix                     : Matrix4<f32>,
    pub viewport_needs_update       : bool,
    pub viewport_dims               : [f32; 2],
}

impl UIState {
    pub fn init() -> Self {

        let ports = serialport::available_ports().expect("No ports found!");

        // let mut connection : Option<(usize, usize)> = None;
        let connection : Option<(usize, GRBLConnection)> = None;

        let dialog_open = Arc::new(AtomicBool::new(false));
        let open_path = Arc::new(std::sync::Mutex::new(imgui::ImString::new(String::new())));

        let baud_rate_i = 7usize;
        let baud_rate = 115_200i32;

        let spindle_rpm_setpoint = 7200;
        let spindle_on = false;

        let gcode_programs = Arc::new(std::sync::Mutex::new(vec![]));
        let active_program : Option<GcodeProgram> = None;

        let machine_coords = [0.0; 3];
        let work_coords = [0.0; 3];


        let main_loop_start = Instant::now();


        let scale = 1.0;
        let center = Vector3::new(0.0, 0.0, 0.0);
        let orientation = Quaternion::from_arc(Vector3::unit_z(), Vector3::unit_z(), None);
        let viewport_needs_update = true;


        UIState {
            ports,
            connection,
            dialog_open,
            open_path,
            baud_rate_i,
            baud_rate,
            spindle_rpm_setpoint,
            spindle_on,
            gcode_programs,
            active_program,
            machine_coords,
            work_coords,
            main_loop_start,
            scale,
            center,
            orientation,
            tmatrix : Matrix4::identity(),
            viewport_needs_update,
            viewport_dims : [256.0, 256.0],
        }
    }


    pub fn frame(&mut self, ui : &mut imgui::Ui, async_runtime : &mut tokio::runtime::Runtime, line_renderer : &mut GCodeRenderer, win : &Window) {
        use imgui::*;
        use imgui::im_str;

        let wdth = win.inner_size().width as f32;
        let hght = win.inner_size().height as f32;

        if let Some(tok) = ui.begin_main_menu_bar() {
            
            if let Some(tok) = ui.begin_menu(im_str!("File"), true) {

                if MenuItem::new(im_str!("Open")).build(ui) {

                    if !self.dialog_open.fetch_or(true, Ordering::SeqCst) {

                        let dialog_open = self.dialog_open.clone();
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
                    .build_simple_string(ui, &mut self.baud_rate_i, &baud_rates_str);

                if self.baud_rate_i == baud_rates.len() {
                    ui.input_int(im_str!("Other##baud_rate_input"), &mut self.baud_rate)
                        .build();
                } else {
                    self.baud_rate = baud_rates[self.baud_rate_i];
                }

                ui.separator();

                if let Some((j, _)) = self.connection {
                    for (i, p) in self.ports.iter().enumerate() {
                        ui.text(&format!("[{:2}] {:?}", i, p.port_name));
                        if i == j {
                            ui.same_line(ww - 80.0);
                            if ui.small_button(im_strf!("Disconnect##{}", p.port_name)) {
                                println!("Disconnected {}", p.port_name);
                                self.connection = None;
                            }
                        }
                    }
                } else {
                    for (i, p) in self.ports.iter().enumerate() {
                        ui.text(&format!("[{:2}] {:?}", i, p.port_name));
                        ui.same_line(ww - 80.0);
                        if ui.small_button(im_strf!("Connect##{}", p.port_name)) {
                            println!("Connected {}", p.port_name);
                            if let Ok(connection) = GRBLConnection::open(&p.port_name, self.baud_rate as u32) {
                                self.connection = Some((i, connection));
                            }
                        }
                    }
                }

                ui.separator();

                if ui.button(im_str!("Refresh"), [80.0, 20.0]) {
                    self.ports = serialport::available_ports().expect("No ports found!");
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
                    
                    if !self.dialog_open.fetch_or(true, Ordering::SeqCst) {
                        let dialog_open = self.dialog_open.clone();
                        let gcode_programs = self.gcode_programs.clone();
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
                

                let active_path = self.active_program.as_ref().map(|p| p.filepath.clone());

                self.gcode_programs.clone().lock().unwrap().drain_filter(|program| {

                    if Some(program.filepath.clone()) == active_path {
                        ui.text(format!("[{:?}]", program.filepath.file_name().unwrap()));
                    } else {
                        ui.text(format!(" {:?} ", program.filepath.file_name().unwrap()));
                    }

                    ui.same_line(ui.window_content_region_width() - 64.0);

                    let load_id = ImString::from(format!("Load##{:?}", program.filepath));
                    let del_id = ImString::from(format!("X##{:?}", program.filepath));

                    if ui.small_button(&load_id) {
                        self.active_program = Some(program.clone());
                        line_renderer.create_line_buffer(&self.active_program.as_ref().unwrap().motionpath);
                        self.viewport_needs_update = true;
                    }

                    ui.same_line(ui.window_content_region_width() - 16.0);
                    
                    if ui.small_button(&del_id) {

                        if self.active_program.as_ref().map(|ap| ap.filepath == program.filepath).unwrap_or(false) {
                            self.active_program = None;
                        }

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

                ui.input_float3(im_str!("Machine Coords"), &mut self.machine_coords)
                    .build();
                ui.input_float3(im_str!("Work Coords"), &mut self.machine_coords)
                    .build();


                ui.separator();
                ui.text(if self.spindle_on {"Spindle (on)"} else {"Spindle (off)"});
                ui.separator();

                Slider::new(im_str!("Target RPM"))
                    .range(7200..=24000)
                    .build(ui, &mut self.spindle_rpm_setpoint);

                self.spindle_rpm_setpoint = (self.spindle_rpm_setpoint as f32 / 100.0).round() as i32 * 100;

                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        if self.spindle_on {[0.0, 0.1, 0.0, 1.0]} else {[0.0, 0.5, 0.0, 1.0]}),
                    (StyleColor::ButtonActive,  if self.spindle_on {[0.0, 0.1, 0.0, 1.0]} else {[0.0, 0.75, 0.0, 1.0]}),
                    (StyleColor::ButtonHovered, if self.spindle_on {[0.0, 0.1, 0.0, 1.0]} else {[0.0, 1.0, 0.0, 1.0]})
                ]);
                self.spindle_on |= ui.button(im_str!("On"), [ww / 2.0 - 10.0, 24.0]);
                tok.pop(ui);

                ui.same_line(ww / 2.0 + 10.0);

                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        if self.spindle_on {[0.5, 0.0, 0.0, 1.0]} else {[0.1, 0.0, 0.0, 1.0]}),
                    (StyleColor::ButtonActive,  if self.spindle_on {[0.75, 0.0, 0.0, 1.0]} else {[0.1, 0.0, 0.0, 1.0]}),
                    (StyleColor::ButtonHovered, if self.spindle_on {[1.0, 0.0, 0.0, 1.0]} else {[0.1, 0.0, 0.0, 1.0]})
                ]);
                self.spindle_on &= !ui.button(im_str!("Off"), [ww / 2.0 - 10.0, 24.0]);
                tok.pop(ui);    

                ui.separator();

                ui.separator();
                ui.text("Jogging");
                ui.separator();

                ui.separator();
                ui.text("Move to Point");
                ui.separator();

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
                let dims = ui.content_region_max();

                self.viewport_dims = dims;


                if ui.is_window_hovered() {
                    let io = ui.io();
                    self.scale *= 1.1f32.powf(io.mouse_wheel);
                    
                    if io.mouse_wheel.abs() < 0.1 {
                        self.viewport_needs_update = true;
                    }
                    let mouse_delta = io.mouse_delta;
                    if io.mouse_down[0] {
                        self.center += (1.0 / (self.scale * dim[0])) * (self.orientation.invert() * Vector3::new(mouse_delta[0], mouse_delta[1], 0.0));

                        if Vector2::from(mouse_delta).magnitude() < 1.0 {
                            self.viewport_needs_update = true;
                        }
                    }
                    if io.mouse_down[1] {
                        let axis = Vector3::new(-mouse_delta[1], mouse_delta[0], 0.0);
                        let angle = axis.magnitude() * 0.001;
                        self.orientation = (Quaternion::from_axis_angle(axis, Rad(angle)) * self.orientation).normalize();
                        if Vector2::from(mouse_delta).magnitude() < 1.0 {
                            self.viewport_needs_update = true;
                        }
                    }
                }

                self.tmatrix = 
                    Matrix4::from_nonuniform_scale(1.0, dims[0] / dims[1], 1.0) * 
                    Matrix4::from_translation(Vector3::new(0.0, 0.0, 0.5)) *
                    Matrix4::from_nonuniform_scale(self.scale, self.scale, 0.0001) * 
                    Matrix4::from(self.orientation) *
                    Matrix4::from_translation(self.center);
                
                if let Some(tid) = line_renderer.texture_id {
                    Image::new(tid, dim)
                        .build(ui);
                }
            });

        tok.pop(ui);
    }
}
