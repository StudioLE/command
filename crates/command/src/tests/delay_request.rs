use crate::prelude::*;

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct DelayRequest {
    name: String,
    pub(super) duration: u64,
}

impl DelayRequest {
    #[must_use]
    pub fn new(name: String, duration: u64) -> Self {
        Self { name, duration }
    }
}

impl Default for DelayRequest {
    fn default() -> Self {
        Self::new("Delay 50 ms".to_owned(), 50)
    }
}

impl Display for DelayRequest {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{}", self.name)
    }
}

impl Executable for DelayRequest {
    type Response = ();
    type ExecutionError = Infallible;
}
