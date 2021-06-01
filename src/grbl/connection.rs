

use std::time::Duration;
use serialport::SerialPort;
use std::collections::VecDeque;
use pest::Parser;

// use std::{pin::Pin, task::{Context, Poll}, time::Duration};
// use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

// use tokio_util::codec::{Decoder, Encoder, Framed};

// use bytes::BytesMut;

use super::*;

// struct GRBLCodec;

// impl Encoder<GRBLCommand> for GRBLCodec {
//     type Error = tokio::io::Error;

//     fn encode(&mut self, item: GRBLCommand, dst: &mut BytesMut) -> Result<(), Self::Error> {

//         dst.extend(item.to_bytes().iter());

//         Ok(())
//     }
// }

// impl Decoder for GRBLCodec {
//     type Item = GRBLStatus;
//     type Error = Box<dyn std::error::Error>;

//     fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        
//         Ok(None)
//     }
// }


// struct AsyncSerialPort(Box<dyn serialport::SerialPort>);

// impl AsyncRead for AsyncSerialPort {
//     fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<tokio::io::Result<()>> {
//         let mut rbuf = [0; 1024];
//         let n = self.0.read(&mut rbuf)?;
//         buf.initialize_unfilled_to(n).copy_from_slice(&rbuf[..n]);

//         Poll::Ready(Ok(()))
//     }
// }

// impl AsyncWrite for AsyncSerialPort {
//     fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, tokio::io::Error>> {

//         Poll::Ready(self.0.write(buf).into())
//     }

//     fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), tokio::io::Error>> {
//         Poll::Ready(Ok(()))
//     }

//     fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), tokio::io::Error>> {
//         Poll::Ready(Ok(()))
//     }
// }

use std::sync::Arc;
use std::sync::Mutex;


pub struct GRBLConnection {
    port : Arc<Mutex<Box<dyn SerialPort>>>,
    command_queue : VecDeque<GRBLCommand>,
    send_line : bool,
    machine_status : GRBLStatus,
    alarm : Option<u8>,
    settings : Option<GRBLSettings>,
}

use std::error::Error;

impl GRBLConnection {
    pub fn open(path : &str, baud_rate : u32) -> Result<Self, Box<dyn Error>> {

        let port = serialport::new(path, baud_rate)
            .timeout(Duration::from_millis(500))
            .open()?;

        let port = Arc::new(Mutex::new(port));

        Ok(Self {
            port,
            command_queue : VecDeque::new(),
            send_line : false,
            machine_status : GRBLStatus::default(),
            alarm : None,
            settings : None,
        })
    }

    pub fn queue_command(&mut self, command : GRBLCommand) {
        self.command_queue.push_back(command);
    }

    pub fn poll(&mut self) -> Result<(), Box<dyn Error>> {

        let mut buf = [0;1024];
        let n = self.port.lock().unwrap().read(&mut buf)?;

        println!("Received: {}", String::from_utf8(buf[..n].to_vec()).unwrap());

        Ok(())
    }

    pub fn execute_realtime_command(&mut self, command : GRBLRealtimeCommand) {
        self.port.lock().unwrap().write(&[command as u8]).unwrap();
    }

    pub fn handle_message(&mut self, offset : usize) -> Option<usize> {
        let s = std::str::from_utf8(&[]).expect("utf-8 encoding error");

        
        if let Ok(mut parsed) = GRBLParser::parse(Rule::line, s) {

            if let Some(msg) = parsed.next() {

                let consumed = msg.as_span().end();

                match msg.as_rule() {
                    Rule::response_message => {
                        let msg = msg.into_inner().next()?;
                        self.send_line = true; 
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