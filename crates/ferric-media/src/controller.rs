use async_trait::async_trait;
use ferric_core::{
    command::Command,
    controller::{ControlError, Controller, Handled},
};

pub struct MediaController;

impl MediaController {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MediaController {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Controller for MediaController {
    fn name(&self) -> &'static str {
        "media"
    }

    async fn execute(&self, _cmd: &Command) -> Result<Handled, ControlError> {
        #[cfg(windows)]
        {
            crate::windows::execute(_cmd).await
        }

        #[cfg(not(windows))]
        {
            Ok(Handled::Unsupported)
        }
    }
}
