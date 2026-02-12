use nix::sys::signal::{self, SigHandler, Signal};

use crate::process::ProcessError;

pub fn setup_signal_handlers() -> Result<(), ProcessError> {
    unsafe {
        signal::signal(Signal::SIGINT, SigHandler::SigIgn)
            .map_err(|e| ProcessError::SignalError(e.to_string()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setup_signal_handlers() -> Result<(), ProcessError> {
        setup_signal_handlers()?;
        Ok(())
    }
}
