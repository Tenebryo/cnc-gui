
use super::*;

#[derive(Parser)]
#[grammar = "grammars/grbl.pest"]
pub struct GRBLParser;


use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct AxisMask : u8 {
        const X = 0b001;
        const Y = 0b010;
        const Z = 0b100;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct StatusReportMask : u8 {
        const MACHINE_POS = 0b001;
        const BUFFER_DATA = 0b010;
    }
}


#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum GRBLRealtimeCommand {
    SoftReset  = 0x18,
    StatusQuery = b'?',
    CycleStartOrResume = b'~',
    FeedHold = b'!',
    SafetyDoor = 0x84,
    JogCancel  = 0x85,
    FeedOverrideReset = 0x90,
    FeedOverrideInc10 = 0x91,
    FeedOverrideDec10 = 0x92,
    FeedOverrideInc01 = 0x93,
    FeedOverrideDec01 = 0x94,
    RapidOverrideFull = 0x95,
    RapidOverrideHalf = 0x96,
    RapidOverrideQuarter = 0x97,
    SpindleOverrideReset = 0x99,
    SpindleOverrideInc10 = 0x9A,
    SpindleOverrideDec10 = 0x9B,
    SpindleOverrideInc01 = 0x9C,
    SpindleOverrideDec01 = 0x9D,
    SpindleToggle = 0x9E,
    FloodToggle = 0xA0,
    MistToggle = 0xA1,
}

#[derive(Debug, Clone)]
pub enum GRBLCommand {
    QuerySettings,        // "$$"
    QueryGCodeParameters, // "$#"
    QueryParserState,     // "$G"
    QueryBuildInfo,       // "$I"
    QueryStartupBlcoks,   // "$N"
    SetStartupBlock {     // "$Nx=line"
        index : u16,
        line : String,
    },
    CheckGCodeMode,       // "$C"
    Setting {             // "$x=y"
        setting : u32,
        value : String,
    },
    KillAlarmLock,        // "$X"
    RunHomingCycle,       // "$H"
    Jog {                 // "$J="
        x : Option<f32>,
        y : Option<f32>,
        z : Option<f32>,
        feed : f32,
        incremental : bool,
        machine_coords : bool
    },
    ResetSettings,        // "$RST=$"
    ResetGCodeParameter,  // "$RST=#"
    ResetGRBL,            // "$RST=*"
    Sleep,                // "$SLP"
}

impl GRBLCommand {
    pub fn to_bytes(self) -> Vec<u8> {
        match self {
            GRBLCommand::QuerySettings        => {b"$$\n"    .to_vec()}
            GRBLCommand::QueryGCodeParameters => {b"$#\n"    .to_vec()}
            GRBLCommand::QueryParserState     => {b"$G\n"    .to_vec()}
            GRBLCommand::QueryBuildInfo       => {b"$I\n"    .to_vec()}
            GRBLCommand::QueryStartupBlcoks   => {b"$N\n"    .to_vec()}
            GRBLCommand::CheckGCodeMode       => {b"$C\n"    .to_vec()}
            GRBLCommand::KillAlarmLock        => {b"$X\n"    .to_vec()}
            GRBLCommand::RunHomingCycle       => {b"$H\n"    .to_vec()}
            GRBLCommand::ResetSettings        => {b"$RST=$\n".to_vec()}
            GRBLCommand::ResetGCodeParameter  => {b"$RST=#\n".to_vec()}
            GRBLCommand::ResetGRBL            => {b"$RST=*\n".to_vec()}
            GRBLCommand::Sleep                => {b"$SLP\n"  .to_vec()}
            GRBLCommand::SetStartupBlock {index,line,} => {
                format!("$N{}={}", index, line).into_bytes()
            }
            GRBLCommand::Jog{x, y, z, feed, incremental, machine_coords,} => {
                format!("$J={}{}{}{}{}F{:.6}\n",
                    if machine_coords {"G53"} else {""},
                    if incremental {"G91"} else {"G90"},
                    x.map(|x| format!("X{:.6}", x)).unwrap_or_default(),
                    y.map(|y| format!("Y{:.6}", y)).unwrap_or_default(),
                    z.map(|z| format!("Z{:.6}", z)).unwrap_or_default(),
                    feed
                ).into_bytes()
            }
            GRBLCommand::Setting{setting, value,} => {
                format!("${}={:.6}\n", setting, value).into_bytes()
            }
            _ => unimplemented!()
        }
    }
}


pub enum GRBLMessage {
    StatusMessage {
        mstate         : Option<GRBLState>,
        mpos           : Option<[f64; 3]>,
        wpos           : Option<[f64; 3]>,
        wco            : Option<bool>,
        buffer_state   : Option<bool>,
        line_number    : Option<bool>,
        feed           : Option<bool>,
        feed_and_speed : Option<bool>,
        inputs         : Option<bool>,
        overrides      : Option<bool>,
    },
    FeedbackMessage,
    AlarmMessage,
    StartupLine,
    WelcomeMessage,
    SettingsMessage,
}
