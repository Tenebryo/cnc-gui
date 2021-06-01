
use std::path::PathBuf;

use cgmath::{Basis2, Deg, Matrix2, Rad, Vector2, prelude::*};
use cgmath::Vector3;

pub type Vec3 = Vector3<f32>;

use crate::gcode;


macro_rules! g {
    ($major:expr) => {
        ('G', _, $major, 0)
    };
    ($major:expr, $minor:expr) => {
        ('G', _, $major, $minor)
    };
}
macro_rules! m {
    ($major:pat) => {
        ('M', _, $major, 0)
    };
    ($major:expr, $minor:expr) => {
        ('M', _, $major, $minor)
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MotionType {
    Rapid,
    Linear,
}

#[derive(Debug, Clone, Copy)]
pub struct MotionPoint {
    pub ty : MotionType,
    pub pos : Vector3<f32>,
    pub time : f32,
}

impl Default for MotionPoint {
    fn default() -> Self {
        MotionPoint {
            ty : MotionType::Linear,
            pos : Vector3::zero(),
            time : 0.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GcodeProgram {
    pub filepath : PathBuf,
    pub program : String,
    pub motionpath : Vec<MotionPoint>,
}

impl GcodeProgram {
    pub fn load(path : PathBuf, program : String) -> GcodeProgram {
        let segments = gcode_to_path_segments(&program);

        let mut lp = Vector3::new(0.0, 0.0, 0.0);
        let speed = 400.0;
        let motionpath = segments.into_iter()
            .map(|mp| {

                let mp = MotionPoint {
                    time : (mp.pos - lp).magnitude() / speed,
                    ..mp
                };
                lp = mp.pos;
                mp
            })
            .collect::<Vec<_>>();

        GcodeProgram {
            filepath: path,
            program,
            motionpath,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionMode {
    G0, G1, G2, G3
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DistanceMode {
    Absolute, Relative,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MotionPlane {
    XY, XZ, YZ
}

#[derive(Debug, Clone)]
struct SimulationState {
    spindle_speed : f32,
    feed_rate : f32,
    rapid_rate : f32,
    motion_mode : MotionMode,
    distance_mode : DistanceMode,
    motion_plane : MotionPlane,
    position : Vec3,
    coord_system : Vec3,
    coord_offset : Vec3,
}

pub fn gcode_to_path_segments(nc : &str) -> Vec<MotionPoint> {

    let lines = gcode::parse(nc);

    let mut path = vec![];

    let home_position = Vec3::new(100., 100., 100.0);

    const MIN_ARC_SEGMENT : f32 = 0.1;

    let mut state = SimulationState{
        spindle_speed : 0.0,
        feed_rate : 0.0,
        rapid_rate : 400.0,
        motion_mode : MotionMode::G0,
        distance_mode : DistanceMode::Absolute,
        motion_plane : MotionPlane::XY,
        position : Vec3::zero(),
        coord_system : Vec3::zero(),
        coord_offset : Vec3::zero(),
    };

    path.push(MotionPoint{pos : state.position, ..Default::default()});

    for (_, l) in lines.iter().enumerate() {

        for word in l.words.iter() {
            match word {

                // rapid | linear move
                g!(0) => {
                    state.motion_mode = MotionMode::G0;
                }

                g!(1) => {
                    state.motion_mode = MotionMode::G1;
                }
                // CW | CCW arc
                g!(2) => {
                    state.motion_mode = MotionMode::G2;
                }
                g!(3) => {
                    state.motion_mode = MotionMode::G3;
                } 

                // dwell
                g!(4) => {} 

                // Set offset (L2 and L10)
                g!(10) => {} 

                // plane selection
                g!(17) => {state.motion_plane = MotionPlane::XY;}
                g!(18) => {state.motion_plane = MotionPlane::XZ;}
                g!(19) => {state.motion_plane = MotionPlane::YZ;}

                // units mode (mm/inch)
                g!(20) => {unimplemented!("G20")}
                g!(21) => {}

                g!(28) => {
                    state.position.z = home_position.z;
                    path.push(MotionPoint{pos : state.position, ty: MotionType::Rapid, ..Default::default()});
                    state.position.x = home_position.x;
                    state.position.y = home_position.y;
                    path.push(MotionPoint{pos : state.position, ty: MotionType::Rapid, ..Default::default()});
                }
                g!(28, 1) => {unimplemented!("G28.1")}

                g!(30, 1) => {unimplemented!("G30.1")}

                g!(38, 2) => {unimplemented!("G38.2")}
                g!(38, 3) => {unimplemented!("G38.3")}
                g!(38, 4) => {unimplemented!("G38.4")}
                g!(38, 5) => {unimplemented!("G38.5")}

                // cutter radius compensation
                g!(40) => {}

                // tool length offset
                g!(49) => {unimplemented!("G49")}
                g!(43, 1) => {unimplemented!("G43.1")}

                g!(53) => {unimplemented!("G53")}

                // coordinate system select
                g!(54) => {}
                g!(55) => {}
                g!(56) => {}
                g!(57) => {}
                g!(58) => {}
                g!(59) => {}


                g!(80) => {unimplemented!("G80")}

                // distance mode (absolute or relative)
                g!(90) => {state.distance_mode = DistanceMode::Absolute;}
                g!(91) => {state.distance_mode = DistanceMode::Relative;}
                g!(91, 1) => {unimplemented!("G91.1")}

                // 
                g!(92) => {}
                g!(92, 1) => {}

                // feedrate mode
                g!(93) => {}
                g!(94) => {}
                g!(95) => {}

                // program mode
                m!(0) => {}
                m!(1) => {}
                m!(2) => {}
                m!(30) => {}

                // spindle state
                m!(3) => {}
                m!(4) => {}
                m!(5) => {}

                // coolant state
                m!(7) => {}
                m!(8) => {}
                m!(9) => {}

                _ => {
                    
                }
            }
        }

        match state.motion_mode {
            MotionMode::G0 | MotionMode::G1 => {

                let start = state.position;

                if state.distance_mode == DistanceMode::Absolute {
                    if let Some(x) = l.value_for('X') { state.position.x = x; }
                    if let Some(y) = l.value_for('Y') { state.position.y = y; }
                    if let Some(z) = l.value_for('Z') { state.position.z = z; }
                } else {
                    if let Some(x) = l.value_for('X') { state.position.x += x; }
                    if let Some(y) = l.value_for('Y') { state.position.y += y; }
                    if let Some(z) = l.value_for('Z') { state.position.z += z; }
                }

                let end = state.position;
                let ty = if state.motion_mode == MotionMode::G0 {MotionType::Rapid} else {MotionType::Linear};

                path.push(MotionPoint{pos : start * 0.975 + end * 0.025, ty, ..Default::default()});
                path.push(MotionPoint{pos : start * 0.025 + end * 0.975, ty, ..Default::default()});                
                path.push(MotionPoint{pos : end,                         ty, ..Default::default()});
            }
            MotionMode::G2 | MotionMode::G3 => {

                let dir = if state.motion_mode == MotionMode::G2 {
                    -1.0
                } else {
                    1.0
                };

                fn swizzle(v : Vector3<f32>, p : MotionPlane) -> Vector3<f32> {
                    match p {
                        MotionPlane::XY => v,
                        MotionPlane::XZ => Vector3::new(v.z, v.x, v.y),
                        MotionPlane::YZ => Vector3::new(v.y, v.z, v.x),
                    }
                }
                fn unswizzle(v : Vector3<f32>, p : MotionPlane) -> Vector3<f32> {
                    match p {
                        MotionPlane::XY => v,
                        MotionPlane::XZ => Vector3::new(v.y, v.z, v.x),
                        MotionPlane::YZ => Vector3::new(v.z, v.x, v.y),
                    }
                }

                let plane = state.motion_plane;

                let start_center = Vector3::new(
                    l.value_for('I').unwrap_or(0.0),
                    l.value_for('J').unwrap_or(0.0),
                    l.value_for('K').unwrap_or(0.0),
                );
                let start_center = swizzle(start_center, plane).truncate();

                let start = swizzle(state.position, plane);

                let center = start + start_center.extend(0.0);

                if state.distance_mode == DistanceMode::Absolute {
                    if let Some(x) = l.value_for('X') { state.position.x = x; }
                    if let Some(y) = l.value_for('Y') { state.position.y = y; }
                    if let Some(z) = l.value_for('Z') { state.position.z = z; }
                } else {
                    if let Some(x) = l.value_for('X') { state.position.x += x; }
                    if let Some(y) = l.value_for('Y') { state.position.y += y; }
                    if let Some(z) = l.value_for('Z') { state.position.z += z; }
                }

                let end = swizzle(state.position, plane);

                let center_end = (end - center).truncate();

                assert!((start_center.magnitude() - center_end.magnitude()).abs() < 0.01);

                let rotations = l.value_for('P').unwrap_or(1.0);

                let angle = dir * (-start_center).angle(center_end).0;

                use core::f32::consts::*;

                let partial_angle = if angle < 0.0 {
                    PI + PI + angle
                } else {
                    angle
                };

                let total_angle = 2.0 * PI * (rotations - 1.0) + partial_angle;

                let center_start = -start_center;

                let segments = (start_center.magnitude() * total_angle / MIN_ARC_SEGMENT).ceil();

                let segment_angle = total_angle / segments;

                let total_z = start.z - end.z;

                assert!(segments >= 1.0 && segments < 1_000_000.0);

                for s in 1..(segments as usize - 1) {

                    let s = s as f32;
                    let pos = center + (Matrix2::from_angle(Rad(dir * segment_angle * s)) * center_start).extend(total_z * (s / segments));

                    path.push(MotionPoint{pos : unswizzle(pos, plane), ..Default::default()});

                }
                
                path.push(MotionPoint{pos : state.position, ..Default::default()});
            }
        }
    }


    path
}