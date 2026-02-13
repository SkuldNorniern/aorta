use crate::process::ProcessError;

pub fn setup_signal_handlers() -> Result<(), ProcessError> {
    // Keep parent shell signal behavior unchanged.
    // External commands should inherit normal signal handling.
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
