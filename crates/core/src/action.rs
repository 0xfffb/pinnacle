/// Suggested enforcement action after risk evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Allow,
    Challenge,
    Block,
}
