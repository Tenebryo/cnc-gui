/*!
 * 
 * This file contains the logic to communicate with GRBL and keep an updated state.
 * 
 */

use std::time::Duration;
use pest::Parser;
use serialport::SerialPort;

use super::*;

pub struct GRBLConnection {
    pub port : Box<dyn SerialPort>,
    pub machine_status : GRBLStatus,
    pub alarm : Option<u8>,
    pub settings : Option<GRBLSettings>,
    pub read_buffer : Vec<u8>,
    pub write_buffer : Vec<u8>,
    pub ready : bool,
    pub error : bool,
}

use std::error::Error;

impl GRBLConnection {
    pub fn open(path : &str, baud_rate : u32) -> Result<Self, Box<dyn Error>> {

        let mut port = serialport::new(path, baud_rate)
            .timeout(Duration::from_millis(100))
            .open()?;

        port.set_timeout(Duration::from_millis(1)).unwrap();

        Ok(Self {
            port,
            machine_status : GRBLStatus::default(),
            alarm : None,
            settings : None,
            read_buffer : vec![],
            write_buffer : vec![],
            ready : true,
            error : false,
        })
    }

    pub fn send_message(&mut self, msg : String) -> Result<(), Box<dyn Error>> {

        self.ready = false;
        self.write_buffer.extend(msg.bytes());

        Ok(())
    }

    pub fn poll(&mut self) -> Result<(), Box<dyn Error>> {

        // read up to 1024 chars
        let mut buf = [0;1024];
        let n = match self.port.read(&mut buf) {
            Ok(n) => {n}
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {0}
            Err(e) => {return Err(e.into());}
        };

        let mut read_buffer = core::mem::replace(&mut self.read_buffer, vec![]);

        read_buffer.extend_from_slice(&buf[..n]);

        //check if the message is complete
        if let Some(n) = self.handle_message(std::str::from_utf8(&read_buffer).expect("utf-8 encoding error")) {
            read_buffer.drain(0..n);
        }

        self.read_buffer = read_buffer;

        if self.write_buffer.len() > 0 {

            // write message if available
            let n = match self.port.write(&self.write_buffer) {
                Ok(n) => {n}
                Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {0}
                Err(e) => {return Err(e.into());}
            };

            // self.write_buffer.drain(0..n);
            let sent = self.write_buffer.drain(0..n).collect::<Vec<_>>();

            println!("sent: {:?}", std::str::from_utf8(&sent).unwrap());
        }

        Ok(())
    }

    pub fn execute_realtime_command(&mut self, command : GRBLRealtimeCommand) {
        self.port.write_all(&[command as u8]).unwrap();
    }

    pub fn handle_message(&mut self, s : &str) -> Option<usize> {
        
        if let Ok(mut parsed) = GRBLParser::parse(Rule::line, s) {

            if let Some(msg) = parsed.next() {

                let consumed = msg.as_span().end();

                // if msg.as_rule() != Rule::unrecognized_message {
                //     println!("processed message: {:?}: {:?}", msg.as_rule(), msg.as_str());
                // }

                match msg.as_rule() {
                    Rule::response_message => {
                        let msg = msg.into_inner().next()?;
                        self.ready = true; 
                        match msg.as_rule() {
                            Rule::ok => {}
                            Rule::error => {
                                // log the error type
                            }
                            _ => unreachable!()
                        }
                    }
                    Rule::push_message => {
                        let msg = msg.into_inner().next()?;
                        match msg.as_rule() {
                            Rule::status_message => {
                                for item in msg.into_inner() {
                                    match item.as_rule() {
                                        Rule::mstate       => {
                                            let inner = item.into_inner().next().unwrap();
                                            let last = inner.as_str().chars().last().unwrap();
                                            match inner.as_rule() {
                                                Rule::idle  => {self.machine_status.state = GRBLState::Idle}
                                                Rule::run   => {self.machine_status.state = GRBLState::Run}
                                                Rule::hold  => {self.machine_status.state = GRBLState::Hold(last == '1')}
                                                Rule::jog   => {self.machine_status.state = GRBLState::Jog}
                                                Rule::alarm => {self.machine_status.state = GRBLState::Alarm}
                                                Rule::door  => {self.machine_status.state = GRBLState::Door(last as u8 - b'0')}
                                                Rule::check => {self.machine_status.state = GRBLState::Check}
                                                Rule::home  => {self.machine_status.state = GRBLState::Home}
                                                Rule::sleep => {self.machine_status.state = GRBLState::Sleep}
                                                _ => unreachable!()
                                            }
                                        }
                                        Rule::mpos         => {
                                            let mut inner = item.into_inner();
                                            let x = inner.next().unwrap().as_str().parse::<f32>().unwrap();
                                            let y = inner.next().unwrap().as_str().parse::<f32>().unwrap();
                                            let z = inner.next().unwrap().as_str().parse::<f32>().unwrap();

                                            self.machine_status.machine_position = [x,y,z];
                                        }
                                        Rule::wpos         => {
                                            let mut inner = item.into_inner();
                                            let x = inner.next().unwrap().as_str().parse::<f32>().unwrap();
                                            let y = inner.next().unwrap().as_str().parse::<f32>().unwrap();
                                            let z = inner.next().unwrap().as_str().parse::<f32>().unwrap();

                                            let [wx,wy,wz] = self.machine_status.work_offset;

                                            self.machine_status.machine_position = [x+wx,y+wy,z+wz];
                                        }
                                        Rule::wco          => {
                                            let mut inner = item.into_inner();
                                            let x = inner.next().unwrap().as_str().parse::<f32>().unwrap();
                                            let y = inner.next().unwrap().as_str().parse::<f32>().unwrap();
                                            let z = inner.next().unwrap().as_str().parse::<f32>().unwrap();

                                            self.machine_status.work_offset = [x,y,z];
                                        }
                                        Rule::buffer_state => {
                                            let mut inner = item.into_inner();
                                            let blocks = inner.next().unwrap().as_str().parse::<u32>().unwrap();
                                            let bytes = inner.next().unwrap().as_str().parse::<u32>().unwrap();

                                            self.machine_status.buffer_free_blocks = blocks;
                                            self.machine_status.buffer_free_bytes = bytes;
                                        }
                                        Rule::line_number  => {
                                            let mut inner = item.into_inner();
                                            let n = inner.next().unwrap().as_str().parse::<u32>().unwrap();

                                            self.machine_status.line_number = n;
                                        }
                                        Rule::feed         => {
                                            let mut inner = item.into_inner();
                                            let feed = inner.next().unwrap().as_str().parse::<f32>().unwrap();

                                            self.machine_status.feed = feed;
                                        }
                                        Rule::feed_and_speed => {
                                            let mut inner = item.into_inner();
                                            let feed = inner.next().unwrap().as_str().parse::<f32>().unwrap();
                                            let speed = inner.next().unwrap().as_str().parse::<f32>().unwrap();

                                            self.machine_status.feed = feed;
                                            self.machine_status.speed = speed;
                                        }
                                        Rule::inputs       => {}
                                        Rule::overrides    => {
                                            let mut inner = item.into_inner();
                                            let f = inner.next().unwrap().as_str().parse::<u32>().unwrap();
                                            let r = inner.next().unwrap().as_str().parse::<u32>().unwrap();
                                            let s = inner.next().unwrap().as_str().parse::<u32>().unwrap();

                                            self.machine_status.override_feed = f;
                                            self.machine_status.override_rapid = r;
                                            self.machine_status.override_speed = s;
                                        }
                                        Rule::accessories  => {}
                                        _ => unreachable!()
                                    }
                                }
                            }
                            Rule::feedback_message => {}
                            Rule::alarm_message => {
                                let last = msg.as_str().chars().last().unwrap() as u8;
                                self.alarm = Some(last - b'0');
                            }
                            Rule::startup_line => {
                                //log
                                println!("received GBRL startup");
                            }
                            Rule::welcome_message => {}
                            Rule::settings_message => {}
                            _ => unreachable!()
                        }
                    }
                    _ => {}
                }
                
                return Some(consumed);
            }
        }
        None
    }


}