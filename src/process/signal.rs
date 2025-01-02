use crate::process::ProcessError;

use libc::{signal, sighandler_t, SIGINT};

pub extern "C" fn handle_sigint(_: i32) {
    // Do nothing, let the child process handle the signal
}

pub fn setup_signal_handlers() -> Result<(), ProcessError> {
    unsafe {
        signal(SIGINT, handle_sigint as sighandler_t);
    }
    Ok(())
}
