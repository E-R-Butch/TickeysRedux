//! Main-thread input recovery policy, kept independent of Cocoa for regression tests.

pub trait Monitor {
    fn is_enabled(&self) -> bool;
    fn set_enabled(&self, enabled: bool);
}

#[derive(Debug, Eq, PartialEq)]
pub enum RecoveryOutcome {
    Active,
    PermissionMissing,
    Failed(String),
}

/// Call only outside the event-tap callback: replacing a monitor drops its run-loop source.
pub fn recover_monitor<M: Monitor>(
    monitor: &mut Option<M>,
    permission_granted: bool,
    create: impl FnOnce() -> Result<M, String>,
) -> RecoveryOutcome {
    if !permission_granted {
        // A formerly enabled tap must not be reported as healthy after revocation.
        *monitor = None;
        return RecoveryOutcome::PermissionMissing;
    }
    if let Some(existing) = monitor.as_ref() {
        if !existing.is_enabled() {
            existing.set_enabled(true);
        }
        if existing.is_enabled() {
            return RecoveryOutcome::Active;
        }
    }
    // Destroy the stale source before adding its replacement, so two taps never coexist.
    *monitor = None;
    match create() {
        Ok(replacement) => {
            *monitor = Some(replacement);
            RecoveryOutcome::Active
        }
        Err(error) => RecoveryOutcome::Failed(error),
    }
}

pub struct RecoveryPolicy {
    sleeping: bool,
    permission_requested: bool,
    retry_attempt: usize,
}

impl RecoveryPolicy {
    pub const fn new() -> Self {
        Self {
            sleeping: false,
            permission_requested: false,
            retry_attempt: 0,
        }
    }

    pub fn is_sleeping(&self) -> bool {
        self.sleeping
    }

    pub fn set_sleeping(&mut self, sleeping: bool) {
        self.sleeping = sleeping;
    }

    pub fn begin_recheck(&mut self) {
        self.retry_attempt = 0;
    }

    pub fn request_permission_once(&mut self) -> bool {
        if self.permission_requested {
            false
        } else {
            self.permission_requested = true;
            true
        }
    }

    pub fn next_retry_delay(&mut self) -> Option<f64> {
        const DELAYS: [f64; 3] = [0.25, 1.0, 3.0];
        if self.sleeping {
            return None;
        }
        let delay = DELAYS.get(self.retry_attempt).copied();
        if delay.is_some() {
            self.retry_attempt += 1;
        }
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    struct FakeMonitor {
        enabled: Cell<bool>,
        can_reenable: bool,
        drops: Rc<Cell<usize>>,
    }

    impl Monitor for FakeMonitor {
        fn is_enabled(&self) -> bool {
            self.enabled.get()
        }

        fn set_enabled(&self, enabled: bool) {
            self.enabled.set(enabled && self.can_reenable);
        }
    }

    impl Drop for FakeMonitor {
        fn drop(&mut self) {
            self.drops.set(self.drops.get() + 1);
        }
    }

    fn monitor(enabled: bool, can_reenable: bool, drops: &Rc<Cell<usize>>) -> FakeMonitor {
        FakeMonitor {
            enabled: Cell::new(enabled),
            can_reenable,
            drops: Rc::clone(drops),
        }
    }

    #[test]
    fn permission_revocation_drops_existing_tap_without_creating_or_prompting() {
        let drops = Rc::new(Cell::new(0));
        let mut current = Some(monitor(true, true, &drops));
        assert_eq!(
            recover_monitor(&mut current, false, || panic!(
                "must not create without permission"
            )),
            RecoveryOutcome::PermissionMissing
        );
        assert!(current.is_none());
        assert_eq!(drops.get(), 1);
    }

    #[test]
    fn healthy_and_reenabled_taps_are_reused() {
        for enabled in [true, false] {
            let drops = Rc::new(Cell::new(0));
            let mut current = Some(monitor(enabled, true, &drops));
            assert_eq!(
                recover_monitor(&mut current, true, || panic!("healthy tap must be reused")),
                RecoveryOutcome::Active
            );
            assert!(current.as_ref().unwrap().is_enabled());
            assert_eq!(drops.get(), 0);
        }
    }

    #[test]
    fn failed_reenable_drops_old_tap_before_creating_replacement() {
        let drops = Rc::new(Cell::new(0));
        let mut current = Some(monitor(false, false, &drops));
        assert_eq!(
            recover_monitor(&mut current, true, || {
                assert_eq!(drops.get(), 1, "old source is removed before replacement");
                Ok(monitor(true, true, &drops))
            }),
            RecoveryOutcome::Active
        );
        assert!(current.as_ref().unwrap().is_enabled());
    }

    #[test]
    fn creation_failure_leaves_slot_empty_and_later_attempt_can_recover() {
        let drops = Rc::new(Cell::new(0));
        let mut current = Some(monitor(false, false, &drops));
        assert_eq!(
            recover_monitor(&mut current, true, || Err("temporarily unavailable".into())),
            RecoveryOutcome::Failed("temporarily unavailable".into())
        );
        assert!(current.is_none());
        assert_eq!(
            recover_monitor(&mut current, true, || Ok(monitor(true, true, &drops))),
            RecoveryOutcome::Active
        );
    }

    #[test]
    fn retries_stop_until_a_new_lifecycle_signal_and_sleep_suppresses_them() {
        let mut policy = RecoveryPolicy::new();
        assert_eq!(policy.next_retry_delay(), Some(0.25));
        assert_eq!(policy.next_retry_delay(), Some(1.0));
        assert_eq!(policy.next_retry_delay(), Some(3.0));
        assert_eq!(policy.next_retry_delay(), None);
        assert_eq!(policy.next_retry_delay(), None);
        policy.set_sleeping(true);
        policy.begin_recheck();
        assert_eq!(policy.next_retry_delay(), None);
        policy.set_sleeping(false);
        assert_eq!(policy.next_retry_delay(), Some(0.25));
    }

    #[test]
    fn denial_does_not_repeat_permission_prompt_on_reactivation_or_wake() {
        let mut policy = RecoveryPolicy::new();
        assert!(policy.request_permission_once());
        for _ in 0..10 {
            policy.begin_recheck();
            policy.set_sleeping(true);
            policy.set_sleeping(false);
            assert!(!policy.request_permission_once());
        }
    }
}
