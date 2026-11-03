/// Outcome of one attempt to present the timer prompt across all displays.
///
/// Internal services use this receipt to distinguish decorative minimal
/// surfaces from the single interactive surface required for user actions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PresentationReceipt {
    pub(crate) windows_shown: usize,
    pub(crate) interactive_shown: bool,
}
