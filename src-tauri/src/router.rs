use std::sync::Arc;

use ferric_core::{
    command::Command,
    controller::{Controller, Handled},
};

const SIGN_IN_REQUIRED: &str = "Sign in to Spotify to use this";

pub struct Router {
    controllers: Vec<Arc<dyn Controller>>,
}

impl Router {
    pub fn new(controllers: Vec<Arc<dyn Controller>>) -> Self {
        Self { controllers }
    }

    pub async fn execute(&self, command: &Command) -> Result<(), String> {
        for controller in &self.controllers {
            match controller.execute(command).await {
                Ok(Handled::Done) => return Ok(()),
                Ok(Handled::Unsupported) => continue,
                Err(error) => {
                    tracing::warn!(
                        controller = controller.name(),
                        %error,
                        "controller command failed"
                    );
                    return Err(error.to_string());
                }
            }
        }

        Err(SIGN_IN_REQUIRED.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use ferric_core::{
        command::Command,
        controller::{ControlError, Controller, Handled},
    };

    use super::{Router, SIGN_IN_REQUIRED};

    struct FakeController {
        name: &'static str,
        result: Result<Handled, ControlError>,
        calls: Arc<Mutex<Vec<&'static str>>>,
    }

    #[async_trait]
    impl Controller for FakeController {
        fn name(&self) -> &'static str {
            self.name
        }

        async fn execute(&self, _command: &Command) -> Result<Handled, ControlError> {
            self.calls.lock().expect("call log lock").push(self.name);
            match &self.result {
                Ok(handled) => Ok(*handled),
                Err(ControlError::NotRunning) => Err(ControlError::NotRunning),
                Err(ControlError::PremiumRequired) => Err(ControlError::PremiumRequired),
                Err(ControlError::NotSignedIn) => Err(ControlError::NotSignedIn),
                Err(ControlError::RateLimited(seconds)) => Err(ControlError::RateLimited(*seconds)),
                Err(ControlError::Other(message)) => Err(ControlError::Other(message.clone())),
            }
        }
    }

    fn fake(
        name: &'static str,
        result: Result<Handled, ControlError>,
        calls: &Arc<Mutex<Vec<&'static str>>>,
    ) -> Arc<dyn Controller> {
        Arc::new(FakeController {
            name,
            result,
            calls: Arc::clone(calls),
        })
    }

    #[tokio::test]
    async fn tries_controllers_in_order_until_one_handles_the_command() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new(vec![
            fake("media", Ok(Handled::Unsupported), &calls),
            fake("spotify", Ok(Handled::Done), &calls),
            fake("unused", Ok(Handled::Done), &calls),
        ]);

        assert_eq!(
            router.execute(&Command::Seek { position_ms: 42 }).await,
            Ok(())
        );
        assert_eq!(
            *calls.lock().expect("call log lock"),
            vec!["media", "spotify"]
        );
    }

    #[tokio::test]
    async fn returns_sign_in_message_when_every_controller_is_unsupported() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new(vec![fake("media", Ok(Handled::Unsupported), &calls)]);

        assert_eq!(
            router.execute(&Command::Seek { position_ms: 42 }).await,
            Err(SIGN_IN_REQUIRED.to_owned())
        );
    }

    #[tokio::test]
    async fn propagates_controller_errors_without_trying_later_controllers() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new(vec![
            fake("media", Err(ControlError::NotRunning), &calls),
            fake("spotify", Ok(Handled::Done), &calls),
        ]);

        assert_eq!(
            router.execute(&Command::PlayPause).await,
            Err("Spotify is not running".to_owned())
        );
        assert_eq!(*calls.lock().expect("call log lock"), vec!["media"]);
    }
}
