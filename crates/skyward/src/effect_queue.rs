//! Drain UI effects in order, including effects returned by their handlers.

use std::collections::VecDeque;

pub fn dispatch<T, E>(effects: Vec<T>, mut handle: impl FnMut(T) -> Result<Vec<T>, E>) -> Result<(), E> {
    let mut queue = VecDeque::from(effects);
    while let Some(effect) = queue.pop_front() {
        queue.extend(handle(effect)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    enum Effect {
        Haptic,
        ScanWlan,
        ReadFiles,
        FollowUp,
    }

    #[test]
    fn timed_navigation_dispatches_non_motor_effects_and_their_answers_in_order() {
        let mut handled = Vec::new();
        let result = dispatch(vec![Effect::Haptic, Effect::ScanWlan, Effect::ReadFiles], |effect| {
            let reply = if effect == Effect::ScanWlan {
                vec![Effect::FollowUp]
            } else {
                Vec::new()
            };
            handled.push(effect);
            Ok::<_, ()>(reply)
        });
        assert_eq!(result, Ok(()));
        assert_eq!(handled, [Effect::Haptic, Effect::ScanWlan, Effect::ReadFiles, Effect::FollowUp]);
    }

    #[test]
    fn dispatch_propagates_handler_errors_without_running_later_effects() {
        let mut handled = Vec::new();
        let result = dispatch(vec![Effect::Haptic, Effect::ScanWlan, Effect::ReadFiles], |effect| {
            let failed = effect == Effect::ScanWlan;
            handled.push(effect);
            if failed { Err("scan unavailable") } else { Ok(Vec::new()) }
        });
        assert_eq!(result, Err("scan unavailable"));
        assert_eq!(handled, [Effect::Haptic, Effect::ScanWlan]);
    }
}
