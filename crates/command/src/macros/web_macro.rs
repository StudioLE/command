use crate::prelude::*;

#[macro_export]
macro_rules! define_commands_web {
    ($($kind:ident($req:ty)),* $(,)?) => {
        #[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
        pub enum CommandRequest {
            $(
                $kind($req),
            )*
        }

        impl IRequest for CommandRequest {}

        $(
            impl From<$req> for CommandRequest {
                fn from(request: $req) -> Self {
                    Self::$kind(request)
                }
            }
        )*

        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub enum CommandSuccess {
            $(
                $kind(<$req as Executable>::Response),
            )*
        }

        impl ISuccess for CommandSuccess {}

        #[derive(Debug)]
        pub enum CommandFailure {
            $(
                $kind(<$req as Executable>::ExecutionError),
            )*
        }

        impl IFailure for CommandFailure {}

        #[derive(Clone, Debug, Deserialize, Serialize)]
        pub struct CommandEvent {
            kind: EventKind,
            request: CommandRequest,
            success: Option<CommandSuccess>,
        }

        impl IEvent<CommandRequest, CommandSuccess> for CommandEvent {
            fn new(kind: EventKind, request: CommandRequest, success: Option<CommandSuccess>) -> Self {
                Self { kind, request, success }
            }

            fn get_kind(&self) -> &EventKind {
                &self.kind
            }

            fn get_request(&self) -> &CommandRequest {
                &self.request
            }

            fn get_success(&self) -> &Option<CommandSuccess> {
                &self.success
            }
        }

        pub struct CommandInfo;

        impl ICommandInfo for CommandInfo {
            type Request = CommandRequest;
            #[cfg(feature = "server")]
            type Command =  Command;
            #[cfg(feature = "server")]
            type Handler = CommandHandler;
            type Success = CommandSuccess;
            type Failure = CommandFailure;
            type Event = CommandEvent;
        }
    };
}

/// Marker trait for serializable command request enums.
pub trait IRequest:
    Clone + Debug + DeserializeOwned + Eq + Hash + PartialEq + Send + Serialize + Sync
{
}

/// Marker trait for serializable command success enums.
pub trait ISuccess: Clone + Debug + DeserializeOwned + Send + Serialize + Sync {}

/// Marker trait for command failure enums.
pub trait IFailure: Debug + Send + Sync {}

/// A command lifecycle event carrying the request and optional success data.
pub trait IEvent<Req: IRequest, S: ISuccess>: Clone + Debug + Send + Sync {
    /// Create a new event.
    fn new(kind: EventKind, request: Req, success: Option<S>) -> Self;
    /// Lifecycle stage of the event.
    fn get_kind(&self) -> &EventKind;
    /// Request that triggered the event.
    fn get_request(&self) -> &Req;
    /// Success data, if the command succeeded.
    fn get_success(&self) -> &Option<S>;
}

/// Associated types that define a complete command system.
pub trait ICommandInfo {
    /// Request enum type.
    type Request: IRequest;
    /// Command enum type (server only).
    #[cfg(feature = "server")]
    type Command: ICommand<Self::Handler, Self::Success, Self::Failure>;
    /// Handler enum type (server only).
    #[cfg(feature = "server")]
    type Handler: IHandler;
    /// Success enum type.
    type Success: ISuccess;
    /// Failure enum type.
    type Failure: IFailure;
    /// Event type.
    type Event: IEvent<Self::Request, Self::Success>;
}
