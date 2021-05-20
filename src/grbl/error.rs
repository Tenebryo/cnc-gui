
#[derive(Debug, Clone)]
pub enum GRBLError {
    
}

use std::fmt;

impl fmt::Display for GRBLError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        
        Ok(())
    }
}

impl std::error::Error for GRBLError {

}
