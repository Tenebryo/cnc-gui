
use cgmath::*;
use imgui::ImString;
use std::time::Instant;
use crate::{WindowRect, gcode_renderer::GCodeRenderer, grbl::{GCodeTaskHandle, GRBLCommand, start_gcode_sender_task}};
use std::sync::Arc;
use std::path::PathBuf;
use std::sync::atomic::Ordering;

use cgmath::{InnerSpace, Quaternion, Rad, Rotation3, Rotation, Vector2, Vector3};
use serialport::SerialPortInfo;

use std::sync::atomic::AtomicBool;

use winit::window::Window;

use crate::simulation::GcodeProgram;

pub struct UIState {
    pub ports                       : Vec<SerialPortInfo>,
    pub connection                  : Option<(usize, GCodeTaskHandle)>,
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
    pub work_coord_system           : usize,
    pub main_loop_start             : Instant,
    pub scale                       : f32,
    pub center                      : Vector3<f32>,
    pub orientation                 : Quaternion<f32>,
    pub tmatrix                     : Matrix4<f32>,
    pub viewport_needs_update       : bool,
    pub viewport_dims               : [f32; 2],
    pub command_input               : ImString,
    pub command_history             : Vec<String>,
    pub previous_frame_end          : Instant,
    pub jog_feed_rate               : f32,
    pub jog_distance                : usize,
}

impl UIState {
    pub fn init() -> Self {

        let ports = serialport::available_ports().expect("No ports found!");

        // let mut connection : Option<(usize, usize)> = None;
        let connection : Option<(usize, GCodeTaskHandle)> = None;

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

        let command_input = ImString::from(String::new());

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
            work_coord_system : 1,
            main_loop_start,
            scale,
            center,
            orientation,
            tmatrix : Matrix4::identity(),
            viewport_needs_update,
            viewport_dims : [256.0, 256.0],
            command_input,
            command_history : vec![],
            previous_frame_end : Instant::now(),
            jog_feed_rate : 200.0,
            jog_distance : 2,
        }
    }


    pub fn frame(&mut self, ui : &mut imgui::Ui, async_runtime : &mut tokio::runtime::Runtime, viewport : &crate::viewport::Viewport, line_renderer : &mut GCodeRenderer, win : &Window) {

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

        let machine_state_h = 256.0;

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
            size : [side_panel_w, hght - menu_bar_h - machine_state_h],
        };

        let state_window_rect = WindowRect{
            pos : [wdth - side_panel_w, hght - machine_state_h],
            size : [side_panel_w, machine_state_h],
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
                                core::mem::replace(&mut self.connection, None).unwrap().1.stop();
                                println!("Disconnected {}", p.port_name);
                            }
                        }
                    }
                } else {
                    for (i, p) in self.ports.iter().enumerate() {
                        ui.text(&format!("[{:2}] {:?}", i, p.port_name));
                        ui.same_line(ww - 80.0);
                        if ui.small_button(im_strf!("Connect##{}", p.port_name)) {
                            let gcode_task_handle = start_gcode_sender_task(p.port_name.clone(), self.baud_rate as u32);
                            self.connection = Some((i, gcode_task_handle));
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

                    let is_active = Some(program.filepath.clone()) == active_path;

                    if is_active {
                        ui.text(format!("[{:?}]", program.filepath.file_name().unwrap()));

                        if let Some((_, ref conn)) = self.connection {
                            ui.same_line(ui.window_content_region_width() - 196.0);

                            ui.text(&format!("{:>6} /", conn.gcode_line.load(Ordering::Relaxed)));
                        }

                        ui.same_line(ui.window_content_region_width() - 128.0);

                        ui.text(format!("{:>6}", program.lines.len()));
                    } else {
                        ui.text(format!(" {:?} ", program.filepath.file_name().unwrap()));

                        ui.same_line(ui.window_content_region_width() - 128.0);

                        ui.text(format!("{:>6}", program.lines.len()));

                    }


                    ui.same_line(ui.window_content_region_width() - 64.0);

                    let load_id = ImString::from(format!("Load##{:?}", program.filepath));
                    let del_id = ImString::from(format!("X##{:?}", program.filepath));

                    if !is_active {
                        if ui.small_button(&load_id) {
                            self.active_program = Some(program.clone());
                            line_renderer.create_line_buffer(&self.active_program.as_ref().unwrap().motionpath);
                            self.viewport_needs_update = true;

                            let mut minx = f32::MAX;
                            let mut miny = f32::MAX;

                            let mut maxx = -f32::MAX;
                            let mut maxy = -f32::MAX;

                            let mut sum = Vector3::new(0.0, 0.0, 0.0);

                            for mp in program.motionpath.iter() {
                                sum += mp.pos;

                                let t = self.tmatrix * mp.pos.extend(1.0);

                                minx = minx.min(t.x);
                                miny = miny.min(t.y);

                                maxx = maxx.max(t.x);
                                maxy = maxy.max(t.y);
                            }

                            sum /= program.motionpath.len() as f32;

                            if maxx - minx > 0.001 || maxy - miny > 0.001 {
                                self.scale *= 1.0 / (maxx-minx).max(maxy-miny);
                                self.center = sum;
                            }
                        }
                    }

                    ui.same_line(ui.window_content_region_width() - 16.0);
                    
                    if ui.small_button(&del_id) {

                        if self.active_program.as_ref().map(|ap| ap.filepath == program.filepath).unwrap_or(false) {
                            line_renderer.clear_line_buffer();
                            self.active_program = None;
                        }

                        return true;
                    }

                    false
                });

                ui.separator();

                if let Some(ref ap) = self.active_program {
                    if let Some((_, ref conn)) = self.connection {
                        if ui.small_button(im_str!("Validate Program")) {
                            conn.validate_program(ap.clone());
                        }
                        if ui.small_button(im_str!("Start Program")) {
                            conn.start_program(ap.clone());
                        }
                        if !conn.paused.load(Ordering::Relaxed) {
                            if ui.small_button(im_str!("Pause Program")) {
                                conn.pause_gcode();
                            }
                        } else {
                            if ui.small_button(im_str!("Unpause Program")) {
                                conn.unpause_gcode();
                            }
                        }
                        if ui.small_button(im_str!("Stop Program")) {
                            conn.stop_program();
                        }
                    }
                }
            });


        // this window contains controls used to give realtime commands
        // to GRBL, such as feed hold, cycle start, and abort.
        imgui::Window::new(im_str!("Command Input"))
            .position(state_window_rect.pos, imgui::Condition::Always)
            .size(state_window_rect.size, imgui::Condition::Always)
            .scroll_bar(true)
            .collapsible(false)
            .resizable(false)
            .build(ui, || {

                if self.connection.is_none() {
                    ui.text("Connect to a controller.");
                    return;
                }

                let hit_enter = ui.input_text(im_str!("##Command Input"), &mut self.command_input)
                    .enter_returns_true(true)
                    .resize_buffer(true)
                    .build();


                if ui.small_button(im_str!("Send Command")) || hit_enter {
                    if let Some((_, ref conn)) = self.connection {
                        conn.send_string(self.command_input.to_string());
                        self.command_history.push(self.command_input.to_string());
                        self.command_input.clear();
                    }
                }

                for (i, cmd) in self.command_history.iter().enumerate() {
                    ui.text(format!("[{}] {}", i, cmd));
                    ui.same_line(state_window_rect.size[0] - 32.0);
                    if ui.small_button(im_strf!("^##[{}]{}",i,cmd)) {
                        self.command_input = ImString::new(cmd);
                    }
                }
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

                if self.connection.is_none() {
                    ui.text("Connect to a controller.");
                    return;
                }

                let [ww, wh] = ui.window_content_region_max();

                let machine_status = self.connection.as_ref().map(|conn| conn.1.get_machine_status()).unwrap_or_default();


                ui.text(format!("Machine State: {:?}", machine_status.state));
                ui.separator();

                match machine_status.state {
                    crate::grbl::GRBLState::Idle    => {}
                    crate::grbl::GRBLState::Run     => {}
                    crate::grbl::GRBLState::Hold(_) => {}
                    crate::grbl::GRBLState::Jog     => {}
                    crate::grbl::GRBLState::Alarm   => {}
                    crate::grbl::GRBLState::Door(_) => {}
                    crate::grbl::GRBLState::Check   => {}
                    crate::grbl::GRBLState::Home    => {}
                    crate::grbl::GRBLState::Sleep   => {}
                }


                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        [0.0, 0.5, 0.0, 1.0]),
                    (StyleColor::ButtonActive,  [0.0, 0.75, 0.0, 1.0]),
                    (StyleColor::ButtonHovered, [0.0, 1.0, 0.0, 1.0])
                ]);


                if ui.button(im_str!("Start##Cycle Start"), [ww / 3.0 - 10.0, 24.0]) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::CycleStartOrResume);
                    }
                }
                tok.pop(ui);

                ui.same_line(ww / 3.0 + 10.0);

                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        [0.5, 0.5, 0.0, 1.0]),
                    (StyleColor::ButtonActive,  [0.75, 0.75, 0.0, 1.0]),
                    (StyleColor::ButtonHovered, [0.9, 0.9, 0.0, 1.0])
                ]);
                if ui.button(im_str!("Hold##Feed Hold"), [ww / 3.0 - 10.0, 24.0]) {

                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::FeedHold);
                    }
                }
                tok.pop(ui);

                ui.same_line(2.0 * ww / 3.0 + 10.0);

                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        [0.5, 0.0, 0.0, 1.0]),
                    (StyleColor::ButtonActive,  [0.75, 0.0, 0.0, 1.0]),
                    (StyleColor::ButtonHovered, [1.0, 0.0, 0.0, 1.0])
                ]);
                if ui.button(im_str!("Reset##Reset"), [ww / 3.0 - 10.0, 24.0]) {

                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::SoftReset);
                    }
                }


                tok.pop(ui);

                ui.separator();
                ui.text("Tool Position");
                ui.separator();


                self.machine_coords = machine_status.machine_position;
                for i in 0..3 {
                    self.work_coords[i] = machine_status.machine_position[i] - machine_status.work_offset[i];
                }

                if ui.input_float3(im_str!("Machine Coords"), &mut self.machine_coords).build() {
                    //set machine offset
                }

                if ui.input_float3(im_str!("Work Coords"), &mut self.work_coords)
                    .enter_returns_true(true)
                    .build() {
                    
                    
                    //set work offset
                    if let Some(ref conn) = self.connection {
                        conn.1.send_string(format!("G10 P0 L2 X{} Y{} Z{}", 
                            self.machine_coords[0] - self.work_coords[0],
                            self.machine_coords[1] - self.work_coords[1],
                            self.machine_coords[2] - self.work_coords[2]
                        ));
                    }
                }

                let mut work_offset = machine_status.work_offset;

                if ui.input_float3(im_str!("Work Offset"), &mut work_offset)
                    .enter_returns_true(true)
                    .build() {
                    //set work offset
                    
                    if let Some(ref conn) = self.connection {
                        conn.1.send_string(format!("G10 P0 L2 X{} Y{} Z{}", 
                            work_offset[0],
                            work_offset[1],
                            work_offset[2]
                        ));
                    }
                }

                let work_coords = [
                    im_str!("G53"),
                    im_str!("G54"),
                    im_str!("G55"),
                    im_str!("G56"),
                    im_str!("G57"),
                    im_str!("G58"),
                    im_str!("G59"),
                ];

                if imgui::ComboBox::new(im_str!("Work Coordinate System")).build_simple_string(ui, &mut self.work_coord_system, &work_coords) {
                    if let Some(ref conn) = self.connection {
                        match self.work_coord_system {
                            0 => {conn.1.send_string(format!("G53"));}
                            1 => {conn.1.send_string(format!("G54"));}
                            2 => {conn.1.send_string(format!("G55"));}
                            3 => {conn.1.send_string(format!("G56"));}
                            4 => {conn.1.send_string(format!("G57"));}
                            5 => {conn.1.send_string(format!("G58"));}
                            6 => {conn.1.send_string(format!("G59"));}
                            _ => panic!()
                        }
                    }
                }

                if let Some(ref conn) = self.connection {
                    if self.work_coord_system != 0 {
                        if ui.small_button(im_str!("X = 0")) {conn.1.send_string(format!("G10 P0 L2 X{}", -machine_status.machine_position[0]));}
                        ui.same_line(64.0);
                        if ui.small_button(im_str!("Y = 0")) {conn.1.send_string(format!("G10 P0 L2 Y{}", -machine_status.machine_position[1]));}
                        ui.same_line(128.0);
                        if ui.small_button(im_str!("Z = 0")) {conn.1.send_string(format!("G10 P0 L2 Z{}", -machine_status.machine_position[2]));}
                        
                        if ui.small_button(im_str!("XY = 0")) {conn.1.send_string(format!("G10 P0 L2 X{} Y{}", -machine_status.machine_position[0], -machine_status.machine_position[1]));}
                        ui.same_line(64.0);
                        if ui.small_button(im_str!("XYZ = 0")) {conn.1.send_string(format!("G10 P0 L2 X{} Y{} Z{}", -machine_status.machine_position[0], -machine_status.machine_position[1], -machine_status.machine_position[2]));}
                    }
                }

                let prev_spindle_on = machine_status.spindle_cw || machine_status.spindle_ccw;

                ui.separator();
                ui.text(if prev_spindle_on {"Spindle (on)"} else {"Spindle (off)"});
                ui.separator();

                Slider::new(im_str!("Target RPM"))
                    .range(7200..=24000)
                    .build(ui, &mut self.spindle_rpm_setpoint);


                self.spindle_on = prev_spindle_on;

                self.spindle_rpm_setpoint = (self.spindle_rpm_setpoint as f32 / 25.0).round() as i32 * 25;

                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        [0.0, 0.5, 0.0, 1.0]),
                    (StyleColor::ButtonActive,  [0.0, 0.75, 0.0, 1.0]),
                    (StyleColor::ButtonHovered, [0.0, 1.0, 0.0, 1.0])
                ]);


                if ui.button(im_str!("On##Spindle On"), [ww / 2.0 - 10.0, 24.0]) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_string(format!("M3 S{}", self.spindle_rpm_setpoint));
                    }
                }

                tok.pop(ui);

                ui.same_line(ww / 2.0 + 10.0);

                let tok = ui.push_style_colors(&[
                    (StyleColor::Button,        [0.5, 0.0, 0.0, 1.0]),
                    (StyleColor::ButtonActive,  [0.75, 0.0, 0.0, 1.0]),
                    (StyleColor::ButtonHovered, [1.0, 0.0, 0.0, 1.0])
                ]);
                if ui.button(im_str!("Off##Spindle Off"), [ww / 2.0 - 10.0, 24.0]) {

                    if let Some(ref conn) = self.connection {
                        conn.1.send_string(format!("M5"));
                    }
                }

                tok.pop(ui);    

                ui.separator();
                ui.text("Overrides");
                ui.separator();

                ui.text(format!("Spindle Override:      {:>3}%", machine_status.override_speed));

                if ui.small_button(im_str!("-10%##Spindle Override -10")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::SpindleOverrideDec10);
                    }
                }
                ui.same_line(48.0);
                if ui.small_button(im_str!("-1%##Spindle Override -1")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::SpindleOverrideDec01);
                    }
                }
                ui.same_line(48.0+32.0);
                if ui.small_button(im_str!("+1%##Spindle Override +1")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::SpindleOverrideInc01);
                    }
                }
                ui.same_line(48.0+32.0+32.0);
                if ui.small_button(im_str!("+10%##Spindle Override +10")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::SpindleOverrideInc10);
                    }
                }
                ui.same_line(48.0+32.0+32.0+40.0);
                if ui.small_button(im_str!("Reset##Spindle Override Reset")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::SpindleOverrideReset);
                    }
                }

                ui.text(format!("Feed Override:         {:>3}%", machine_status.override_feed));

                if ui.small_button(im_str!("-10%##Feed Override -10")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::FeedOverrideDec10);
                    }
                }
                ui.same_line(48.0);
                if ui.small_button(im_str!("-1%##Feed Override -1")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::FeedOverrideDec01);
                    }
                }
                ui.same_line(48.0+32.0);
                if ui.small_button(im_str!("+1%##Feed Override +1")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::FeedOverrideInc01);
                    }
                }
                ui.same_line(48.0+32.0+32.0);
                if ui.small_button(im_str!("+10%##Feed Override +10")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::FeedOverrideInc10);
                    }
                }
                ui.same_line(48.0+32.0+32.0+40.0);
                if ui.small_button(im_str!("Reset##Feed Override Reset")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::FeedOverrideReset);
                    }
                }


                ui.text(format!("Rapid Override:        {:>3}%", machine_status.override_rapid));

                if ui.small_button(im_str!(" 25%##Rapid Override 25")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::RapidOverrideQuarter);
                    }
                }
                ui.same_line(48.0);
                if ui.small_button(im_str!("50%##Rapid Override 50")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::RapidOverrideHalf);
                    }
                }
                ui.same_line(48.0+32.0);
                if ui.small_button(im_str!("100%##Rapid Override 100")) {
                    if let Some(ref conn) = self.connection {
                        conn.1.send_realtime_command(crate::grbl::GRBLRealtimeCommand::RapidOverrideFull);
                    }
                }


                ui.separator();
                ui.text("Jogging");
                ui.separator();

                ui.input_float(im_str!("Jog Feed"), &mut self.jog_feed_rate)
                    .build();

                let jog_distances = [ 0.001, 0.01, 0.1, 1.0, 10.0, 100.0 ];

                imgui::ComboBox::new(im_str!("Jog Distance"))
                    .build_simple(ui, &mut self.jog_distance, &jog_distances, &|x : &f32| std::borrow::Cow::Owned(im_stringf!("{:3.3}", x)));
                
                let jog_distance = jog_distances[self.jog_distance];

                ui.text("Jog X");
                let mut jog_x = 0;
                if ui.small_button(im_str!("   <<   ##Jog X -10")) {jog_x = -10;}
                ui.same_line(8.0 + 1.0*72.0);
                if ui.small_button(im_str!("    <   ##Jog X  -1")) {jog_x =  -1;}
                ui.same_line(8.0 + 2.0*72.0);
                if ui.small_button(im_str!("   >    ##Jog X  10")) {jog_x =   1;}
                ui.same_line(8.0 + 3.0*72.0);
                if ui.small_button(im_str!("   >>   ##Jog X   1")) {jog_x =  10;}

                ui.text("Jog Y");
                let mut jog_y = 0;
                if ui.small_button(im_str!("   vv   ##Jog Y -10")) {jog_y = -10;}
                ui.same_line(8.0 + 1.0*72.0);
                if ui.small_button(im_str!("    v   ##Jog Y  -1")) {jog_y =  -1;}
                ui.same_line(8.0 + 2.0*72.0);
                if ui.small_button(im_str!("   ^    ##Jog Y  10")) {jog_y =   1;}
                ui.same_line(8.0 + 3.0*72.0);
                if ui.small_button(im_str!("   ^^   ##Jog Y   1")) {jog_y =  10;}

                ui.text("Jog Z");
                let mut jog_z = 0;
                if ui.small_button(im_str!("   vv   ##Jog Z -10")) {jog_z = -10;}
                ui.same_line(8.0 + 1.0*72.0);
                if ui.small_button(im_str!("    v   ##Jog Z  -1")) {jog_z =  -1;}
                ui.same_line(8.0 + 2.0*72.0);
                if ui.small_button(im_str!("   ^    ##Jog Z   1")) {jog_z =   1;}
                ui.same_line(8.0 + 3.0*72.0);
                if ui.small_button(im_str!("   ^^   ##Jog Z  10")) {jog_z =  10;}

                if let Some(ref conn) = self.connection {

                    if jog_x != 0 {
                        conn.1.send_command(GRBLCommand::Jog{
                            x: Some(jog_distance * jog_x as f32),
                            y: None,
                            z: None,
                            feed: self.jog_feed_rate,
                            incremental: true,
                            machine_coords: false,
                        });
                    }

                    if jog_y != 0 {
                        conn.1.send_command(GRBLCommand::Jog{
                            x: None,
                            y: Some(jog_distance * jog_y as f32),
                            z: None,
                            feed: self.jog_feed_rate,
                            incremental: true,
                            machine_coords: false,
                        });
                    }

                    if jog_z != 0 {
                        conn.1.send_command(GRBLCommand::Jog{
                            x: None,
                            y: None,
                            z: Some(jog_distance * jog_z as f32),
                            feed: self.jog_feed_rate,
                            incremental: true,
                            machine_coords: false,
                        });
                    }
                }


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
                        self.orientation = (Quaternion::from_axis_angle(axis, Rad(-angle)) * self.orientation).normalize();
                        if Vector2::from(mouse_delta).magnitude() < 1.0 {
                            self.viewport_needs_update = true;
                        }
                    }
                }

                self.tmatrix = 
                    Matrix4::from_scale(self.scale) * 
                    Matrix4::from(self.orientation) *
                    Matrix4::from_translation(self.center);
                
                if let Some(tid) = viewport.texture_id {
                    Image::new(tid, dim)
                        .build(ui);
                }
            });

        tok.pop(ui);

        self.previous_frame_end = Instant::now();
    }
}
