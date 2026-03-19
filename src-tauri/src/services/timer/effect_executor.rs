use tracing::info;

use super::effect::Effect;
use crate::services::ServiceContext;

/// Execute timer effects by dispatching to concrete services.
pub(crate) fn execute_effect(app: Option<&ServiceContext>, effect: &Effect) {
    let Some(app) = app else {
        info!("STUB effect: {effect:?}");
        return;
    };

    app.execute_timer_effect(effect);
}
