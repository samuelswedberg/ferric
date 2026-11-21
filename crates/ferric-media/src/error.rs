#[derive(Debug, thiserror::Error)]
pub enum MediaError {
    #[error("media integration is not implemented")]
    Unimplemented,
}
