use super::*;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GRBLState {
    Idle,
    Run,
    Hold(bool),
    Jog,
    Alarm,
    Door(u8),
    Check,
    Home,
    Sleep,
}

impl Default for GRBLState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct GRBLStatus {
    pub machine_position : [f32; 3],
    pub work_offset : [f32; 3],
    pub state : GRBLState,
    pub buffer_free_blocks : u32,
    pub buffer_free_bytes : u32,
    pub line_number : u32,
    pub feed : f32,
    pub speed : f32,
    pub override_feed : u32,
    pub override_speed : u32,
    pub override_rapid : u32,
}



#[derive(Default, Clone, Copy)]
pub struct GRBLSettings {
    pub step_pulse_micros       : u16,              // 0    Step pulse time, microseconds
    pub step_idle_millis        : u8,               // 1    Step idle delay, milliseconds
    pub step_invert_mask        : AxisMask,         // 2    Step pulse invert, mask
    pub direction_invert_mask   : AxisMask,         // 3    Step direction invert, mask
    pub step_enable_invert      : bool,             // 4    Invert step enable pin, boolean
    pub limit_pin_invert        : bool,             // 5    Invert limit pins, boolean
    pub probe_pin_invert        : bool,             // 6    Invert probe pin, boolean
    pub status_report_mask      : StatusReportMask, // 10   Status report options, mask
    pub junction_deviation      : f32,              // 11   Junction deviation, millimeters
    pub arc_tolerance           : f32,              // 12   Arc tolerance, millimeters
    pub report_inches           : bool,             // 13   Report in inches, boolean
    pub soft_limits_enable      : bool,             // 20   Soft limits enable, boolean
    pub hard_limits_enable      : bool,             // 21   Hard limits enable, boolean
    pub homing_cycle_enable     : bool,             // 22   Homing cycle enable, boolean
    pub homing_direction_mask   : AxisMask,         // 23   Homing direction invert, mask
    pub homing_locate_rate      : f32,              // 24   Homing locate feed rate, mm/min
    pub homing_search_rate      : f32,              // 25   Homing search seek rate, mm/min
    pub homing_switch_debounce  : u16,              // 26   Homing switch debounce delay, milliseconds
    pub homing_pulloff_distance : f32,              // 27   Homing switch pull-off distance, millimeters
    pub spindle_max_speed       : f32,              // 30   Maximum spindle speed, RPM
    pub spindle_min_speed       : f32,              // 31   Minimum spindle speed, RPM
    pub laser_mode              : bool,             // 32   Laser-mode enable, boolean
    pub steps_per_mm_x          : f32,              // 100  X-axis steps per millimeter
    pub steps_per_mm_y          : f32,              // 101  Y-axis steps per millimeter
    pub steps_per_mm_z          : f32,              // 102  Z-axis steps per millimeter
    pub max_rate_x              : f32,              // 110  X-axis maximum rate, mm/min
    pub max_rate_y              : f32,              // 111  Y-axis maximum rate, mm/min
    pub max_rate_z              : f32,              // 112  Z-axis maximum rate, mm/min
    pub acceleration_x          : f32,              // 120  X-axis acceleration, mm/sec^2
    pub acceleration_y          : f32,              // 121  Y-axis acceleration, mm/sec^2
    pub acceleration_z          : f32,              // 122  Z-axis acceleration, mm/sec^2
    pub max_travel_x            : f32,              // 130  X-axis maximum travel, millimeters
    pub max_travel_y            : f32,              // 131  Y-axis maximum travel, millimeters
    pub max_travel_z            : f32,              // 132  Z-axis maximum travel, millimeters
}

impl GRBLSettings {
    fn set_all_command_list(&self) -> [GRBLCommand; 34] {
        [
            GRBLCommand::Setting{setting : 0  , value : format!("{}",    self.step_pulse_micros)},            // u16               0  
            GRBLCommand::Setting{setting : 1  , value : format!("{}",    self.step_idle_millis)},             // u8                1  
            GRBLCommand::Setting{setting : 2  , value : format!("{}",    self.step_invert_mask.bits())},        // AxisMask          2  
            GRBLCommand::Setting{setting : 3  , value : format!("{}",    self.direction_invert_mask.bits())},   // AxisMask          3  
            GRBLCommand::Setting{setting : 4  , value : format!("{}",    self.step_enable_invert as u8)},     // bool              4  
            GRBLCommand::Setting{setting : 5  , value : format!("{}",    self.limit_pin_invert as u8)},       // bool              5  
            GRBLCommand::Setting{setting : 6  , value : format!("{}",    self.probe_pin_invert as u8)},       // bool              6  
            GRBLCommand::Setting{setting : 10 , value : format!("{}",    self.status_report_mask.bits())},      // StatusReportMask  10 
            GRBLCommand::Setting{setting : 11 , value : format!("{:.6}", self.junction_deviation)},           // f32               11 
            GRBLCommand::Setting{setting : 12 , value : format!("{:.6}", self.arc_tolerance)},                // f32               12 
            GRBLCommand::Setting{setting : 13 , value : format!("{}",    self.report_inches as u8)},          // bool              13 
            GRBLCommand::Setting{setting : 20 , value : format!("{}",    self.soft_limits_enable as u8)},     // bool              20 
            GRBLCommand::Setting{setting : 21 , value : format!("{}",    self.hard_limits_enable as u8)},     // bool              21 
            GRBLCommand::Setting{setting : 22 , value : format!("{}",    self.homing_cycle_enable as u8)},    // bool              22 
            GRBLCommand::Setting{setting : 23 , value : format!("{}",    self.homing_direction_mask.bits())},   // AxisMask          23 
            GRBLCommand::Setting{setting : 24 , value : format!("{:.6}", self.homing_locate_rate)},           // f32               24 
            GRBLCommand::Setting{setting : 25 , value : format!("{:.6}", self.homing_search_rate)},           // f32               25 
            GRBLCommand::Setting{setting : 26 , value : format!("{}",    self.homing_switch_debounce)},       // u16               26 
            GRBLCommand::Setting{setting : 27 , value : format!("{:.6}", self.homing_pulloff_distance)},      // f32               27 
            GRBLCommand::Setting{setting : 30 , value : format!("{:.6}", self.spindle_max_speed)},            // f32               30 
            GRBLCommand::Setting{setting : 31 , value : format!("{:.6}", self.spindle_min_speed)},            // f32               31 
            GRBLCommand::Setting{setting : 32 , value : format!("{}",    self.laser_mode as u8)},             // bool              32 
            GRBLCommand::Setting{setting : 100, value : format!("{:.6}", self.steps_per_mm_x)},               // f32               100
            GRBLCommand::Setting{setting : 101, value : format!("{:.6}", self.steps_per_mm_y)},               // f32               101
            GRBLCommand::Setting{setting : 102, value : format!("{:.6}", self.steps_per_mm_z)},               // f32               102
            GRBLCommand::Setting{setting : 110, value : format!("{:.6}", self.max_rate_x)},                   // f32               110
            GRBLCommand::Setting{setting : 111, value : format!("{:.6}", self.max_rate_y)},                   // f32               111
            GRBLCommand::Setting{setting : 112, value : format!("{:.6}", self.max_rate_z)},                   // f32               112
            GRBLCommand::Setting{setting : 120, value : format!("{:.6}", self.acceleration_x)},               // f32               120
            GRBLCommand::Setting{setting : 121, value : format!("{:.6}", self.acceleration_y)},               // f32               121
            GRBLCommand::Setting{setting : 122, value : format!("{:.6}", self.acceleration_z)},               // f32               122
            GRBLCommand::Setting{setting : 130, value : format!("{:.6}", self.max_travel_x)},                 // f32               130
            GRBLCommand::Setting{setting : 131, value : format!("{:.6}", self.max_travel_y)},                 // f32               131
            GRBLCommand::Setting{setting : 132, value : format!("{:.6}", self.max_travel_z)},                 // f32               132
        ]
    }

    fn set_one_command(&self, index : u8) -> Option<GRBLCommand> {
        match index {
             0 => Some(GRBLCommand::Setting{setting : 0  , value : format!("{}",    self.step_pulse_micros)}),            // u16               0  
             1 => Some(GRBLCommand::Setting{setting : 1  , value : format!("{}",    self.step_idle_millis)}),             // u8                1  
             2 => Some(GRBLCommand::Setting{setting : 2  , value : format!("{}",    self.step_invert_mask.bits())}),        // AxisMask          2  
             3 => Some(GRBLCommand::Setting{setting : 3  , value : format!("{}",    self.direction_invert_mask.bits())}),   // AxisMask          3  
             4 => Some(GRBLCommand::Setting{setting : 4  , value : format!("{}",    self.step_enable_invert as u8)}),     // bool              4  
             5 => Some(GRBLCommand::Setting{setting : 5  , value : format!("{}",    self.limit_pin_invert as u8)}),       // bool              5  
             6 => Some(GRBLCommand::Setting{setting : 6  , value : format!("{}",    self.probe_pin_invert as u8)}),       // bool              6  
             7 => Some(GRBLCommand::Setting{setting : 10 , value : format!("{}",    self.status_report_mask.bits())}),      // StatusReportMask  10 
             8 => Some(GRBLCommand::Setting{setting : 11 , value : format!("{:.6}", self.junction_deviation)}),           // f32               11 
             9 => Some(GRBLCommand::Setting{setting : 12 , value : format!("{:.6}", self.arc_tolerance)}),                // f32               12 
            10 => Some(GRBLCommand::Setting{setting : 13 , value : format!("{}",    self.report_inches as u8)}),          // bool              13 
            11 => Some(GRBLCommand::Setting{setting : 20 , value : format!("{}",    self.soft_limits_enable as u8)}),     // bool              20 
            12 => Some(GRBLCommand::Setting{setting : 21 , value : format!("{}",    self.hard_limits_enable as u8)}),     // bool              21 
            13 => Some(GRBLCommand::Setting{setting : 22 , value : format!("{}",    self.homing_cycle_enable as u8)}),    // bool              22 
            14 => Some(GRBLCommand::Setting{setting : 23 , value : format!("{}",    self.homing_direction_mask.bits())}),   // AxisMask          23 
            15 => Some(GRBLCommand::Setting{setting : 24 , value : format!("{:.6}", self.homing_locate_rate)}),           // f32               24 
            16 => Some(GRBLCommand::Setting{setting : 25 , value : format!("{:.6}", self.homing_search_rate)}),           // f32               25 
            17 => Some(GRBLCommand::Setting{setting : 26 , value : format!("{}",    self.homing_switch_debounce)}),       // u16               26 
            18 => Some(GRBLCommand::Setting{setting : 27 , value : format!("{:.6}", self.homing_pulloff_distance)}),      // f32               27 
            19 => Some(GRBLCommand::Setting{setting : 30 , value : format!("{:.6}", self.spindle_max_speed)}),            // f32               30 
            20 => Some(GRBLCommand::Setting{setting : 31 , value : format!("{:.6}", self.spindle_min_speed)}),            // f32               31 
            21 => Some(GRBLCommand::Setting{setting : 32 , value : format!("{}",    self.laser_mode as u8)}),             // bool              32 
            22 => Some(GRBLCommand::Setting{setting : 100, value : format!("{:.6}", self.steps_per_mm_x)}),               // f32               100
            23 => Some(GRBLCommand::Setting{setting : 101, value : format!("{:.6}", self.steps_per_mm_y)}),               // f32               101
            24 => Some(GRBLCommand::Setting{setting : 102, value : format!("{:.6}", self.steps_per_mm_z)}),               // f32               102
            25 => Some(GRBLCommand::Setting{setting : 110, value : format!("{:.6}", self.max_rate_x)}),                   // f32               110
            26 => Some(GRBLCommand::Setting{setting : 111, value : format!("{:.6}", self.max_rate_y)}),                   // f32               111
            27 => Some(GRBLCommand::Setting{setting : 112, value : format!("{:.6}", self.max_rate_z)}),                   // f32               112
            28 => Some(GRBLCommand::Setting{setting : 120, value : format!("{:.6}", self.acceleration_x)}),               // f32               120
            29 => Some(GRBLCommand::Setting{setting : 121, value : format!("{:.6}", self.acceleration_y)}),               // f32               121
            31 => Some(GRBLCommand::Setting{setting : 122, value : format!("{:.6}", self.acceleration_z)}),               // f32               122
            32 => Some(GRBLCommand::Setting{setting : 130, value : format!("{:.6}", self.max_travel_x)}),                 // f32               130
            33 => Some(GRBLCommand::Setting{setting : 131, value : format!("{:.6}", self.max_travel_y)}),                 // f32               131
            34 => Some(GRBLCommand::Setting{setting : 132, value : format!("{:.6}", self.max_travel_z)}),                 // f32               132
            _ => None,
        }
    }
}
