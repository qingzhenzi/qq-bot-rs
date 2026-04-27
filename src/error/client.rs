use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientBuildError {
    /// 既没传 `Credentials` 也没传现成的 `Bot`——二选一必填。
    #[error("client builder requires either credentials or a Bot")]
    MissingBot,

    /// 没设事件 handler。
    #[error("client builder requires an event handler")]
    MissingHandler,
}
