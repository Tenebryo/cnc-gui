/*!
 * This file contains the logic for sending GCode and other commands to GRBL.
 * The GCodeTaskHandle acts as an interface to the sender
 * 
 * 
 */

use std::{sync::{Arc, Mutex, atomic::{AtomicBool, AtomicU64, Ordering}, mpsc::*}, thread::JoinHandle, time::{Duration, Instant}};

use crate::simulation::GcodeProgram;

use super::{GRBLCommand, GRBLConnection, GRBLRealtimeCommand, GRBLState, GRBLStatus};


pub struct GCodeTaskHandle {
    pub grbl : Arc<Mutex<GRBLStatus>>,
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

    pub fn stop_program(&self) {
        self.sender.send(GCodeTaskMessage::StopProgram).unwrap();
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

    pub fn send_string(&self, mut cmd : String) -> bool {
        if !self.has_gcode.load(Ordering::SeqCst) {

            cmd += "\r\n";

            self.sender.send(GCodeTaskMessage::SendString(cmd)).unwrap();
            true
        } else {
            false
        }
    }

    pub fn pause_gcode(&self) {

        self.paused.store(true, Ordering::Relaxed);
    }

    pub fn unpause_gcode(&self) {

        self.paused.store(false, Ordering::Relaxed);
    }

    pub fn stop(self) {
        self.sender.send(GCodeTaskMessage::Stop).unwrap();
    }

    pub fn get_machine_status(&self) -> GRBLStatus {
        self.grbl.lock().unwrap().clone()
    }
}

pub enum GCodeTaskMessage {
    StartProgram(GcodeProgram),
    ValidateProgram(GcodeProgram),
    StopProgram,
    RealtimeCommand(GRBLRealtimeCommand),
    SendCommand(GRBLCommand),
    SendString(String),
    Stop,
}

pub fn start_gcode_sender_task(path : String, baud_rate : u32) -> GCodeTaskHandle {

    let (tx,rx) = channel::<GCodeTaskMessage>();
    let paused = Arc::new(AtomicBool::new(false));
    let has_gcode = Arc::new(AtomicBool::new(false));
    let gcode_line = Arc::new(AtomicU64::new(0));

    let mut validating = false;

    let grbl_status = Arc::new(Mutex::new(GRBLStatus::default()));
    let join = {
        let grbl_status = grbl_status.clone();
        let paused = paused.clone();
        let gcode_line = gcode_line.clone();
        let has_gcode = has_gcode.clone();
        std::thread::spawn(move || {
            let mut grbl = GRBLConnection::open(&path, baud_rate).unwrap();

            let mut gcode_iter = None;

            let mut last_status = Instant::now();

            loop {

                if gcode_iter.is_none() {
                    gcode_line.store(0, Ordering::Relaxed);
                }

                if last_status.elapsed() > Duration::from_millis(500) {

                    grbl.execute_realtime_command(GRBLRealtimeCommand::StatusQuery);

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
                            if grbl.machine_status.state != GRBLState::Check {
                                grbl.send_message(String::from_utf8(GRBLCommand::CheckGCodeMode.to_bytes()).unwrap()).unwrap();
                            }

                            gcode_iter = Some(prog.lines.into_iter());

                        }
                        GCodeTaskMessage::StopProgram => {
                            println!("stopped program");
                            gcode_iter = None;
                            has_gcode.store(false, Ordering::Relaxed);
                        }
                        GCodeTaskMessage::RealtimeCommand(rtcmd) => {
                            grbl.execute_realtime_command(rtcmd);
                        }
                        GCodeTaskMessage::SendCommand(cmd) => {
                            grbl.send_message(String::from_utf8(cmd.to_bytes()).unwrap()).unwrap();
                        }
                        GCodeTaskMessage::Stop => {
                            return;
                        }
                        GCodeTaskMessage::SendString(s) => {
                            grbl.send_message(s).unwrap();
                        }
                    }
                }

                has_gcode.store(gcode_iter.is_some(), Ordering::Relaxed);

                let grbl_ready = grbl.ready;
                let grbl_error = grbl.error;

                if validating && grbl_error {
                    gcode_iter = None;
                    grbl.send_message(String::from_utf8(GRBLCommand::CheckGCodeMode.to_bytes()).unwrap()).unwrap();
                    validating = false;
                }

                if !paused.load(Ordering::SeqCst) && grbl_ready {

                    match gcode_iter.as_mut().map(|i| i.next()) {
                        Some(Some(mut line)) =>  {

                            if !line.ends_with("\n") {
                                line += "\n";
                            }

                            grbl.send_message(line).unwrap();
                            gcode_line.fetch_add(1, Ordering::Relaxed);
                        }
                        Some(None) => {
                            gcode_iter = None;

                            if validating {

                                grbl.send_message(String::from_utf8(GRBLCommand::CheckGCodeMode.to_bytes()).unwrap()).unwrap();
                                validating = false;
                            }
                        }
                        None => {}
                    }
                }

                grbl.poll().unwrap();

                *grbl_status.lock().unwrap() = grbl.machine_status.clone();
            }
        })
    };

    GCodeTaskHandle {
        grbl : grbl_status,
        sender: tx,
        paused,
        has_gcode,
        join,
        gcode_line,
    }
}