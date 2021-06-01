/*!
 * This file contains the logic for sending GCode and other commands to GRBL.
 * The GCodeTaskHandle acts as an interface to the sender
 * 
 * 
 */

use std::{sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}, mpsc::*}, thread::JoinHandle, time::{Duration, Instant}};

use crate::simulation::GcodeProgram;

use super::{GRBLCommand, GRBLConnection, GRBLRealtimeCommand, GRBLState};


pub struct GCodeTaskHandle {
    pub grbl : Arc<Mutex<Option<GRBLConnection>>>,
    pub sender : Sender<GCodeTaskMessage>,
    pub paused : Arc<AtomicBool>,
    pub has_gcode : Arc<AtomicBool>,
    pub gcode_line : Arc<AtomicU64>,
    pub join : JoinHandle<()>,
}

impl GCodeTaskHandle {
    pub fn start_program(&self, program : GcodeProgram) -> bool {
        if !self.has_gcode.load(Ordering::SeqCst) {
            self.sender.send(GCodeTaskMessage::StartProgram(program)).unwrap();
            true
        } else {
            false
        }
    }

    pub fn validate_program(&self, program : GcodeProgram) -> bool {
        if !self.has_gcode.load(Ordering::SeqCst) {
            self.sender.send(GCodeTaskMessage::ValidateProgram(program)).unwrap();
            true
        } else {
            false
        }
    }

    pub fn stop_program(&self) -> bool {
        if self.has_gcode.load(Ordering::SeqCst) {
            self.sender.send(GCodeTaskMessage::StopProgram).unwrap();
            true
        } else {
            false
        }
    }

    pub fn send_realtime_command(&self, cmd : GRBLRealtimeCommand) -> bool {
        if !self.has_gcode.load(Ordering::SeqCst) {
            self.sender.send(GCodeTaskMessage::RealtimeCommand(cmd)).unwrap();
            true
        } else {
            false
        }
    }

    pub fn send_command(&self, cmd : GRBLCommand) -> bool {
        if !self.has_gcode.load(Ordering::SeqCst) {
            self.sender.send(GCodeTaskMessage::SendCommand(cmd)).unwrap();
            true
        } else {
            false
        }
    }

    pub fn stop_task(self) {

        self.sender.send(GCodeTaskMessage::Stop).unwrap();

        self.join.join().unwrap();
    }
}

pub enum GCodeTaskMessage {
    StartProgram(GcodeProgram),
    ValidateProgram(GcodeProgram),
    StopProgram,
    RealtimeCommand(GRBLRealtimeCommand),
    SendCommand(GRBLCommand),
    Stop,
}

pub fn start_gcode_sender_task(path : String, baud_rate : u32) -> GCodeTaskHandle {

    let (tx,rx) = channel::<GCodeTaskMessage>();
    let paused = Arc::new(AtomicBool::new(false));
    let has_gcode = Arc::new(AtomicBool::new(false));
    let gcode_line = Arc::new(AtomicU64::new(0));

    let mut validating = false;

    let grbl = Arc::new(Mutex::new(None));
    let join = {
        let grbl = grbl.clone();
        let paused = paused.clone();
        let gcode_line = gcode_line.clone();
        std::thread::spawn(move || {
            let conn = GRBLConnection::open(&path, baud_rate).unwrap();

            grbl.lock().unwrap().insert(conn);

            let mut gcode_iter = None;

            let mut last_status = Instant::now();

            loop {

                if last_status.elapsed() > Duration::from_millis(500) {

                    if let Some(ref mut grbl) = *grbl.lock().unwrap() {
                        grbl.execute_realtime_command(GRBLRealtimeCommand::StatusQuery);
                    }

                    last_status = Instant::now();
                }

                if let Ok(msg) = rx.recv_timeout(Duration::from_millis(1)) {
                    match msg {
                        GCodeTaskMessage::StartProgram(prog) => {
                            gcode_line.store(0, Ordering::Relaxed);
                            gcode_iter = Some(prog.lines.into_iter());
                        }
                        GCodeTaskMessage::ValidateProgram(prog) => {
                            validating = true;

                            gcode_line.store(0, Ordering::Relaxed);
                            if let Some(ref mut grbl) = *grbl.lock().unwrap() {
                                if grbl.machine_status.state != GRBLState::Check {
                                    grbl.send_message(String::from_utf8(GRBLCommand::CheckGCodeMode.to_bytes()).unwrap()).unwrap();
                                }
                            }

                            gcode_iter = Some(prog.lines.into_iter());

                        }
                        GCodeTaskMessage::StopProgram => {
                            gcode_iter = None;
                        }
                        GCodeTaskMessage::RealtimeCommand(rtcmd) => {
                            if let Some(ref mut grbl) = *grbl.lock().unwrap() {
                                grbl.execute_realtime_command(rtcmd);
                            }
                        }
                        GCodeTaskMessage::SendCommand(cmd) => {
                            if let Some(ref mut grbl) = *grbl.lock().unwrap() {
                                grbl.send_message(String::from_utf8(cmd.to_bytes()).unwrap()).unwrap();
                            }
                        }
                        GCodeTaskMessage::Stop => {
                            return;
                        }
                    }
                }

                let grbl_ready = grbl.lock().unwrap().as_ref().map(|grbl| grbl.ready).unwrap_or(false);
                let grbl_error = grbl.lock().unwrap().as_ref().map(|grbl| grbl.error).unwrap_or(false);

                if validating && grbl_error {
                    gcode_iter = None;
                    if let Some(ref mut grbl) = *grbl.lock().unwrap() {
                        grbl.send_message(String::from_utf8(GRBLCommand::CheckGCodeMode.to_bytes()).unwrap()).unwrap();
                    }
                    validating = false;
                }

                if !paused.load(Ordering::SeqCst) && grbl_ready {

                    match gcode_iter.as_mut().map(|i| i.next()) {
                        Some(Some(mut line)) =>  {

                            if !line.ends_with("\n") {
                                line += "\n";
                            }

                            if let Some(ref mut grbl) = *grbl.lock().unwrap() {
                                grbl.send_message(line).unwrap();
                                gcode_line.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                        Some(None) => {
                            gcode_iter = None;

                            if validating {

                                if let Some(ref mut grbl) = *grbl.lock().unwrap() {
                                    grbl.send_message(String::from_utf8(GRBLCommand::CheckGCodeMode.to_bytes()).unwrap()).unwrap();
                                }
                                validating = false;
                            }
                        }
                        None => {}
                    }
                }

                if let Some(ref mut grbl) = *grbl.lock().unwrap() {
                    grbl.poll().unwrap();
                }
            }
        })
    };

    GCodeTaskHandle {
        grbl,
        sender: tx,
        paused,
        has_gcode,
        join,
        gcode_line,
    }
}