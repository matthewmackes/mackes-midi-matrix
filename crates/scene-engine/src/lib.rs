//! Project, scene planning, activation, and safety boundary.

/// Terminal outcome for an activation action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionResult {
    /// Action completed.
    Succeeded,
    /// Action failed.
    Failed,
    /// Action exceeded its deadline.
    TimedOut,
    /// Dependency prevented execution.
    SkippedDependency,
    /// Operator cancelled before send.
    Cancelled,
    /// Sent without read-back verification.
    SentUnverified,
}

/// Aggregate activation outcome counts.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActivationSummary {
    /// Number completed successfully.
    pub succeeded: u32,
    /// Number failed or timed out.
    pub failed: u32,
    /// Number skipped by policy/dependency.
    pub skipped: u32,
    /// Number cancelled before send.
    pub cancelled: u32,
    /// Number sent without read-back verification.
    pub sent_unverified: u32,
}

/// Deterministic bounded action rate limiter using a monotonic fixed window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RateLimiter {
    max_actions: u32,
    window_ticks: u64,
    window_start: u64,
    used: u32,
}

impl RateLimiter {
    /// Creates a limiter; both bounds must be nonzero.
    #[must_use]
    pub const fn new(max_actions: u32, window_ticks: u64, now: u64) -> Option<Self> {
        if max_actions == 0 || window_ticks == 0 {
            None
        } else {
            Some(Self { max_actions, window_ticks, window_start: now, used: 0 })
        }
    }

    /// Attempts to admit one action, resetting only after a complete window.
    pub const fn admit(&mut self, now: u64) -> bool {
        if now.saturating_sub(self.window_start) >= self.window_ticks {
            self.window_start = now;
            self.used = 0;
        }
        if self.used >= self.max_actions {
            false
        } else {
            self.used += 1;
            true
        }
    }

    /// Returns the number of actions admitted in the current window.
    #[must_use]
    pub const fn used(self) -> u32 {
        self.used
    }
}

impl ActivationSummary {
    /// Summarizes terminal action results.
    #[must_use]
    pub fn from_results(results: &[(String, ActionResult)]) -> Self {
        let mut summary = Self::default();
        for (_, result) in results {
            match result {
                ActionResult::Succeeded => summary.succeeded += 1,
                ActionResult::SentUnverified => summary.sent_unverified += 1,
                ActionResult::Failed | ActionResult::TimedOut => summary.failed += 1,
                ActionResult::SkippedDependency => summary.skipped += 1,
                ActionResult::Cancelled => summary.cancelled += 1,
            }
        }
        summary
    }

    /// Returns the number of terminal outcomes represented by the summary.
    #[must_use]
    pub const fn total(self) -> u32 {
        self.succeeded + self.failed + self.skipped + self.cancelled + self.sent_unverified
    }

    /// Returns true only when every action completed with verified success.
    #[must_use]
    pub const fn is_fully_verified(self) -> bool {
        self.total() > 0
            && self.succeeded == self.total()
            && self.failed == 0
            && self.skipped == 0
            && self.cancelled == 0
            && self.sent_unverified == 0
    }
}

/// One deterministic action in a scene activation plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationAction {
    /// Stable action identifier.
    pub id: String,
    /// Human-readable action description.
    pub description: String,
    /// Whether this action requires unsafe mode.
    pub unsafe_action: bool,
    /// Optional prerequisite action ID.
    pub depends_on: Option<String>,
}

/// Resolves one activation alias without guessing between candidates.
///
/// # Errors
///
/// Returns an error when no endpoint matches or when the alias is ambiguous.
pub fn resolve_unique_alias(alias: &str, available: &[(&str, u64)]) -> Result<u64, &'static str> {
    if alias.trim().is_empty() {
        return Err("activation alias must not be empty");
    }
    let matches: Vec<u64> = available
        .iter()
        .filter(|(candidate, _)| *candidate == alias)
        .map(|(_, endpoint)| *endpoint)
        .collect();
    match matches.as_slice() {
        [endpoint] => Ok(*endpoint),
        [] => Err("activation alias is offline or missing"),
        _ => Err("activation alias is ambiguous"),
    }
}

/// Policy applied after an activation action fails or times out.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailurePolicy {
    /// Stop before executing subsequent actions.
    Stop,
    /// Continue independent actions while blocking dependents.
    Continue,
}

/// Compiled scene plan and one result slot per action.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivationPlan {
    /// Ordered actions.
    pub actions: Vec<ActivationAction>,
}

/// Central unsafe-mode and performance-lock state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SafetyController {
    performance_lock: bool,
    unsafe_until: Option<u64>,
}

/// Panic policy output for one armed destination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanicAction {
    /// Destination endpoint identifier.
    pub destination: u64,
    /// Send all-notes-off (CC 123).
    pub all_notes_off: bool,
    /// Send all-sound-off (CC 120).
    pub all_sound_off: bool,
}

/// Source class for a governed mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuditSource {
    /// Interactive local TUI.
    LocalTui,
    /// Local command-line client.
    LocalCli,
    /// Startup restore.
    StartupRestore,
    /// MIDI mapping.
    MidiMapping,
    /// RTP-MIDI peer.
    RtpMidi,
}

/// Risk classification attached to every mutation decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RiskClass {
    /// Ordinary reversible action.
    Normal,
    /// Bulk operation.
    Bulk,
    /// Persistent device write.
    PersistentWrite,
    /// Identity mismatch override.
    IdentityMismatch,
    /// Destructive operation.
    Destructive,
}

/// Operator confirmation requirement for a mutation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationClass {
    /// No additional confirmation.
    Normal,
    /// Confirmation for a bulk operation.
    Bulk,
    /// Exact confirmation for persistent device writes.
    PersistentWrite,
    /// Exact identity-mismatch confirmation.
    IdentityMismatch,
    /// Destructive-action confirmation.
    Destructive,
}

/// Governed operation class evaluated by the central safety policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GovernedOperation {
    /// Activate or navigate an ordinary scene.
    SceneChange,
    /// Read monitoring, status, or diagnostics.
    Monitor,
    /// Execute the safe panic plan.
    Panic,
    /// Edit routing, mappings, or project configuration.
    ConfigurationEdit,
    /// Edit or install a device profile.
    ProfileEdit,
    /// Send a hazardous or persistent device action.
    HazardousSend,
    /// Arm local volatile unsafe mode.
    ArmUnsafe,
}

/// Explicit result of centralized mutation authorization.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PolicyDecision {
    /// Operation is authorized.
    Allow,
    /// Performance Lock prohibits this operation.
    DenyPerformanceLock,
    /// Source class can never request this operation.
    DenySource,
    /// Locally armed unsafe mode is required.
    DenyUnsafeMode,
    /// Required operator confirmation was absent.
    DenyConfirmation,
}

/// Returns whether a mutation may proceed under the supplied confirmation.
#[must_use]
pub const fn confirmation_allows(class: ConfirmationClass, confirmed: bool) -> bool {
    matches!(class, ConfirmationClass::Normal) || confirmed
}

/// Structured, payload-redacted mutation audit record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditRecord {
    /// Monotonic timestamp.
    pub timestamp: u64,
    /// Local actor identity.
    pub actor: String,
    /// Origin source class.
    pub source: AuditSource,
    /// Stable action ID.
    pub action_id: String,
    /// Target alias.
    pub target_alias: String,
    /// Risk classification.
    pub risk: RiskClass,
    /// Policy decision.
    pub allowed: bool,
    /// Safe result summary; raw payloads are never stored.
    pub result: String,
}

impl AuditRecord {
    /// Produces a safe result field for an audit record.
    #[must_use]
    pub fn result_summary(result: &str, sensitive: bool) -> String {
        if sensitive {
            "<redacted>".into()
        } else {
            result.to_owned()
        }
    }
}

/// Bounded audit sink retaining the newest governed decisions.
#[derive(Clone, Debug)]
pub struct AuditLog {
    capacity: usize,
    records: Vec<AuditRecord>,
}

impl AuditLog {
    /// Creates a log; zero capacity is rejected to avoid false retention.
    ///
    /// # Errors
    ///
    /// Returns an error when `capacity` is zero.
    pub fn new(capacity: usize) -> Result<Self, &'static str> {
        (capacity > 0)
            .then_some(Self { capacity, records: Vec::new() })
            .ok_or("audit capacity must be positive")
    }

    /// Appends a record and evicts the oldest record when full.
    pub fn append(&mut self, record: AuditRecord) {
        self.records.push(record);
        if self.records.len() > self.capacity {
            self.records.remove(0);
        }
    }

    /// Returns records newest first for operator inspection.
    #[must_use]
    pub fn newest_first(&self) -> impl DoubleEndedIterator<Item = &AuditRecord> {
        self.records.iter().rev()
    }
}

/// Builds a panic action list without reset `SysEx` or persistent writes.
#[must_use]
pub fn panic_plan(destinations: &[u64]) -> Vec<PanicAction> {
    destinations
        .iter()
        .copied()
        .map(|destination| PanicAction { destination, all_notes_off: true, all_sound_off: true })
        .collect()
}

impl SafetyController {
    /// Enables or disables the performance lock.
    pub const fn set_performance_lock(&mut self, locked: bool) {
        self.performance_lock = locked;
    }
    /// Returns whether performance lock is active.
    #[must_use]
    pub const fn performance_locked(self) -> bool {
        self.performance_lock
    }
    /// Arms unsafe mode until the supplied monotonic deadline.
    pub const fn arm_unsafe(&mut self, until_tick: u64) {
        self.unsafe_until = Some(until_tick);
    }
    /// Disarms unsafe mode immediately.
    pub const fn disarm_unsafe(&mut self) {
        self.unsafe_until = None;
    }
    /// Returns true only while unsafe mode is armed and unexpired.
    pub fn unsafe_armed(&mut self, now_tick: u64) -> bool {
        if self.unsafe_until.is_some_and(|deadline| now_tick >= deadline) {
            self.unsafe_until = None;
        }
        self.unsafe_until.is_some()
    }
    /// Clears all volatile safety state, as required on daemon restart.
    pub const fn restart_clear(&mut self) {
        self.performance_lock = false;
        self.unsafe_until = None;
    }

    /// Authorizes one operation through the central performance-lock, source, unsafe-mode,
    /// and confirmation policy.
    #[must_use]
    pub const fn authorize(
        self,
        source: AuditSource,
        operation: GovernedOperation,
        unsafe_armed: bool,
        confirmation: ConfirmationClass,
        confirmed: bool,
    ) -> PolicyDecision {
        if matches!(operation, GovernedOperation::ArmUnsafe)
            && !matches!(source, AuditSource::LocalTui | AuditSource::LocalCli)
        {
            return PolicyDecision::DenySource;
        }
        if self.performance_lock
            && matches!(
                operation,
                GovernedOperation::ConfigurationEdit
                    | GovernedOperation::ProfileEdit
                    | GovernedOperation::HazardousSend
                    | GovernedOperation::ArmUnsafe
            )
        {
            return PolicyDecision::DenyPerformanceLock;
        }
        if matches!(operation, GovernedOperation::HazardousSend) && !unsafe_armed {
            return PolicyDecision::DenyUnsafeMode;
        }
        if !confirmation_allows(confirmation, confirmed) {
            return PolicyDecision::DenyConfirmation;
        }
        PolicyDecision::Allow
    }

    /// Authorizes one operation and appends its redacted decision to the audit sink.
    ///
    /// This is the preferred integration boundary for daemon callers: policy evaluation and
    /// its audit record occur together, so a caller cannot accidentally omit a governed event.
    #[allow(clippy::too_many_arguments)]
    pub fn authorize_and_record(
        &mut self,
        timestamp: u64,
        actor: impl Into<String>,
        source: AuditSource,
        operation: GovernedOperation,
        action_id: impl Into<String>,
        target_alias: impl Into<String>,
        risk: RiskClass,
        confirmation: ConfirmationClass,
        confirmed: bool,
        result: &str,
        sensitive_result: bool,
        audit: &mut AuditLog,
    ) -> PolicyDecision {
        let armed = self.unsafe_armed(timestamp);
        let decision = self.authorize(source, operation, armed, confirmation, confirmed);
        audit.append(AuditRecord {
            timestamp,
            actor: actor.into(),
            source,
            action_id: action_id.into(),
            target_alias: target_alias.into(),
            risk,
            allowed: decision == PolicyDecision::Allow,
            result: AuditRecord::result_summary(result, sensitive_result),
        });
        decision
    }
}

impl ActivationPlan {
    /// Compiles and validates a plan, rejecting duplicate IDs and unknown dependencies.
    ///
    /// # Errors
    ///
    /// Returns a validation error for duplicate IDs or dangling dependencies.
    pub fn compile(actions: Vec<ActivationAction>) -> Result<Self, &'static str> {
        for (index, action) in actions.iter().enumerate() {
            if action.id.trim().is_empty()
                || actions[..index].iter().any(|prior| prior.id == action.id)
            {
                return Err("activation IDs must be unique and non-empty");
            }
            if action.depends_on.as_ref().is_some_and(|dependency| {
                !actions.iter().any(|candidate| candidate.id == *dependency)
            }) {
                return Err("activation dependency is unknown");
            }
        }
        Ok(Self { actions })
    }

    /// Resolves one endpoint alias for each planned action in order.
    ///
    /// # Errors
    ///
    /// Returns an error when the alias count differs from the plan or any
    /// alias is missing or ambiguous.
    pub fn resolve_action_targets(
        &self,
        aliases: &[&str],
        available: &[(&str, u64)],
    ) -> Result<Vec<u64>, &'static str> {
        if aliases.len() != self.actions.len() {
            return Err("activation target count does not match action count");
        }
        aliases.iter().map(|alias| resolve_unique_alias(alias, available)).collect()
    }

    /// Produces terminal results in one-to-one action order, honoring cancellation and unsafe policy.
    #[must_use]
    pub fn execute(&self, unsafe_armed: bool, cancelled: bool) -> Vec<(String, ActionResult)> {
        self.execute_with_policy(unsafe_armed, cancelled, FailurePolicy::Continue, |_| {
            ActionResult::Succeeded
        })
    }

    /// Executes a plan with an injected action callback after policy checks.
    #[must_use]
    pub fn execute_with<F>(
        &self,
        unsafe_armed: bool,
        cancelled: bool,
        execute_action: F,
    ) -> Vec<(String, ActionResult)>
    where
        F: FnMut(&ActivationAction) -> ActionResult,
    {
        self.execute_with_policy(unsafe_armed, cancelled, FailurePolicy::Continue, execute_action)
    }

    /// Cancels every action that has not been sent, returning one terminal result per action.
    ///
    /// This explicit operation is used by panic handling and preserves the planner invariant
    /// that cancellation never claims to undo an action that was already transmitted.
    #[must_use]
    pub fn cancel_unsent(&self) -> Vec<(String, ActionResult)> {
        self.actions.iter().map(|action| (action.id.clone(), ActionResult::Cancelled)).collect()
    }

    /// Executes through a caller-supplied monotonic deadline.
    ///
    /// Once the deadline is reached, no further executor calls are made and every remaining
    /// action receives the explicit `TimedOut` terminal result.
    #[must_use]
    pub fn execute_with_deadline<F>(
        &self,
        unsafe_armed: bool,
        cancelled: bool,
        now: u64,
        deadline: u64,
        mut execute_action: F,
    ) -> Vec<(String, ActionResult)>
    where
        F: FnMut(&ActivationAction) -> ActionResult,
    {
        if cancelled {
            return self.cancel_unsent();
        }
        let mut results = Vec::with_capacity(self.actions.len());
        for action in &self.actions {
            if now >= deadline {
                results.push((action.id.clone(), ActionResult::TimedOut));
            } else if action.unsafe_action && !unsafe_armed {
                results.push((action.id.clone(), ActionResult::SkippedDependency));
            } else {
                results.push((action.id.clone(), execute_action(action)));
            }
        }
        results
    }

    /// Executes a plan with an explicit failure policy.
    #[must_use]
    pub fn execute_with_policy<F>(
        &self,
        unsafe_armed: bool,
        cancelled: bool,
        policy: FailurePolicy,
        mut execute_action: F,
    ) -> Vec<(String, ActionResult)>
    where
        F: FnMut(&ActivationAction) -> ActionResult,
    {
        let mut results = Vec::with_capacity(self.actions.len());
        for action in &self.actions {
            let stopped = policy == FailurePolicy::Stop
                && results.iter().any(|(_, result)| {
                    matches!(result, ActionResult::Failed | ActionResult::TimedOut)
                });
            let dependency_blocked = action.depends_on.as_ref().is_some_and(|dependency| {
                results.iter().any(|(id, result)| {
                    id == dependency
                        && !matches!(result, ActionResult::Succeeded | ActionResult::SentUnverified)
                })
            });
            let result = if cancelled {
                ActionResult::Cancelled
            } else if stopped || dependency_blocked || (action.unsafe_action && !unsafe_armed) {
                ActionResult::SkippedDependency
            } else {
                execute_action(action)
            };
            results.push((action.id.clone(), result));
        }
        results
    }

    /// Executes actions through a bounded global rate limiter.
    #[must_use]
    pub fn execute_with_limiter<F>(
        &self,
        unsafe_armed: bool,
        cancelled: bool,
        limiter: &mut RateLimiter,
        now: u64,
        mut execute_action: F,
    ) -> Vec<(String, ActionResult)>
    where
        F: FnMut(&ActivationAction) -> ActionResult,
    {
        self.execute_with_policy(unsafe_armed, cancelled, FailurePolicy::Continue, |action| {
            if limiter.admit(now) {
                execute_action(action)
            } else {
                ActionResult::Failed
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn action(id: &str, unsafe_action: bool, depends_on: Option<&str>) -> ActivationAction {
        ActivationAction {
            id: id.into(),
            description: id.into(),
            unsafe_action,
            depends_on: depends_on.map(str::to_owned),
        }
    }

    #[test]
    fn dry_plan_and_execution_have_one_result_each() {
        let plan = ActivationPlan::compile(vec![
            action("route", false, None),
            action("write", true, Some("route")),
        ])
        .expect("plan");
        assert_eq!(
            plan.execute(false, false),
            vec![
                ("route".into(), ActionResult::Succeeded),
                ("write".into(), ActionResult::SkippedDependency)
            ]
        );
        assert_eq!(plan.execute(true, false).len(), plan.actions.len());
    }

    #[test]
    fn injected_executor_controls_action_outcome() {
        let plan = ActivationPlan::compile(vec![action("write", false, None)]).expect("plan");
        let results = plan.execute_with(true, false, |_| ActionResult::SentUnverified);
        assert_eq!(results, vec![("write".into(), ActionResult::SentUnverified)]);
    }

    #[test]
    fn blocked_dependency_propagates_to_descendants() {
        let plan = ActivationPlan::compile(vec![
            action("unsafe", true, None),
            action("child", false, Some("unsafe")),
        ])
        .expect("plan");
        assert_eq!(plan.execute(false, false)[1].1, ActionResult::SkippedDependency);
    }

    #[test]
    fn explicit_unsent_cancellation_is_one_to_one_and_never_succeeds() {
        let plan = ActivationPlan::compile(vec![
            action("first", false, None),
            action("second", true, Some("first")),
        ])
        .expect("plan");
        assert_eq!(
            plan.cancel_unsent(),
            vec![
                ("first".into(), ActionResult::Cancelled),
                ("second".into(), ActionResult::Cancelled),
            ]
        );
    }

    #[test]
    fn deadline_stops_executor_and_marks_remaining_actions_timed_out() {
        let plan = ActivationPlan::compile(vec![
            action("first", false, None),
            action("second", false, None),
        ])
        .expect("plan");
        let mut calls = 0;
        let results = plan.execute_with_deadline(false, false, 10, 10, |_| {
            calls += 1;
            ActionResult::Succeeded
        });
        assert_eq!(calls, 0);
        assert_eq!(results[0].1, ActionResult::TimedOut);
        assert_eq!(results[1].1, ActionResult::TimedOut);
        let results = plan.execute_with_deadline(false, false, 9, 10, |_| {
            calls += 1;
            ActionResult::Succeeded
        });
        assert_eq!(calls, 2);
        assert!(results.iter().all(|(_, result)| *result == ActionResult::Succeeded));
    }

    #[test]
    fn failure_policy_stops_or_continues_deterministically() {
        let plan = ActivationPlan::compile(vec![
            action("first", false, None),
            action("second", false, None),
        ])
        .expect("plan");
        let stopped = plan.execute_with_policy(false, false, FailurePolicy::Stop, |item| {
            if item.id == "first" {
                ActionResult::Failed
            } else {
                ActionResult::Succeeded
            }
        });
        assert_eq!(stopped[1].1, ActionResult::SkippedDependency);
        let continued = plan.execute_with_policy(false, false, FailurePolicy::Continue, |item| {
            if item.id == "first" {
                ActionResult::Failed
            } else {
                ActionResult::Succeeded
            }
        });
        assert_eq!(continued[1].1, ActionResult::Succeeded);
    }

    #[test]
    fn unsafe_mode_expires_and_restart_clears_state() {
        let mut safety = SafetyController::default();
        safety.set_performance_lock(true);
        safety.arm_unsafe(10);
        assert!(safety.unsafe_armed(9));
        assert!(!safety.unsafe_armed(10));
        safety.arm_unsafe(20);
        safety.restart_clear();
        assert!(!safety.performance_locked());
        assert!(!safety.unsafe_armed(0));
    }

    #[test]
    fn central_policy_enforces_lock_source_unsafe_and_confirmation_matrix() {
        let mut safety = SafetyController::default();
        for operation in
            [GovernedOperation::SceneChange, GovernedOperation::Monitor, GovernedOperation::Panic]
        {
            assert_eq!(
                safety.authorize(
                    AuditSource::MidiMapping,
                    operation,
                    false,
                    ConfirmationClass::Normal,
                    false,
                ),
                PolicyDecision::Allow
            );
        }
        safety.set_performance_lock(true);
        for operation in [
            GovernedOperation::ConfigurationEdit,
            GovernedOperation::ProfileEdit,
            GovernedOperation::HazardousSend,
            GovernedOperation::ArmUnsafe,
        ] {
            assert_eq!(
                safety.authorize(
                    AuditSource::LocalTui,
                    operation,
                    true,
                    ConfirmationClass::Normal,
                    true,
                ),
                PolicyDecision::DenyPerformanceLock
            );
        }
        safety.set_performance_lock(false);
        for source in [AuditSource::MidiMapping, AuditSource::RtpMidi, AuditSource::StartupRestore]
        {
            assert_eq!(
                safety.authorize(
                    source,
                    GovernedOperation::ArmUnsafe,
                    false,
                    ConfirmationClass::Normal,
                    false,
                ),
                PolicyDecision::DenySource
            );
        }
        assert_eq!(
            safety.authorize(
                AuditSource::LocalCli,
                GovernedOperation::HazardousSend,
                false,
                ConfirmationClass::Destructive,
                true,
            ),
            PolicyDecision::DenyUnsafeMode
        );
        assert_eq!(
            safety.authorize(
                AuditSource::LocalCli,
                GovernedOperation::HazardousSend,
                true,
                ConfirmationClass::Destructive,
                false,
            ),
            PolicyDecision::DenyConfirmation
        );
        assert_eq!(
            safety.authorize(
                AuditSource::LocalCli,
                GovernedOperation::HazardousSend,
                true,
                ConfirmationClass::Destructive,
                true,
            ),
            PolicyDecision::Allow
        );
    }

    #[test]
    fn panic_plan_targets_all_destinations_with_only_safe_controls() {
        assert_eq!(
            panic_plan(&[2, 5]),
            vec![
                PanicAction { destination: 2, all_notes_off: true, all_sound_off: true },
                PanicAction { destination: 5, all_notes_off: true, all_sound_off: true },
            ]
        );
    }

    #[test]
    fn rate_limiter_bounds_actions_and_resets_on_window() {
        assert!(RateLimiter::new(0, 10, 0).is_none());
        let mut limiter = RateLimiter::new(2, 10, 100).expect("limiter");
        assert!(limiter.admit(100));
        assert!(limiter.admit(109));
        assert!(!limiter.admit(109));
        assert_eq!(limiter.used(), 2);
        assert!(limiter.admit(110));
        assert_eq!(limiter.used(), 1);
        let plan =
            ActivationPlan::compile(vec![action("one", false, None), action("two", false, None)])
                .expect("plan");
        let mut execution_limiter = RateLimiter::new(1, 100, 0).expect("limiter");
        let results = plan.execute_with_limiter(false, false, &mut execution_limiter, 0, |_| {
            ActionResult::Succeeded
        });
        assert_eq!(
            results,
            vec![("one".into(), ActionResult::Succeeded), ("two".into(), ActionResult::Failed)]
        );
    }

    #[test]
    fn activation_alias_resolution_rejects_missing_and_ambiguous_endpoints() {
        let available = [("arena", 1), ("lexicon", 2)];
        assert_eq!(resolve_unique_alias("arena", &available), Ok(1));
        assert_eq!(
            resolve_unique_alias("offline", &available),
            Err("activation alias is offline or missing")
        );
        assert_eq!(resolve_unique_alias("", &available), Err("activation alias must not be empty"));
        assert_eq!(
            resolve_unique_alias("arena", &[("arena", 1), ("arena", 3)]),
            Err("activation alias is ambiguous")
        );
        let plan =
            ActivationPlan::compile(vec![action("a", false, None), action("b", false, Some("a"))])
                .expect("plan");
        assert_eq!(plan.resolve_action_targets(&["arena", "lexicon"], &available), Ok(vec![1, 2]));
        assert_eq!(
            plan.resolve_action_targets(&["arena"], &available),
            Err("activation target count does not match action count")
        );
    }

    #[test]
    fn confirmation_policy_requires_confirmation_for_elevated_risk() {
        assert!(confirmation_allows(ConfirmationClass::Normal, false));
        assert!(!confirmation_allows(ConfirmationClass::Bulk, false));
        assert!(confirmation_allows(ConfirmationClass::Destructive, true));
    }

    #[test]
    fn audit_log_is_bounded_and_newest_first() {
        let mut log = AuditLog::new(2).expect("capacity");
        for timestamp in 1..=3 {
            log.append(AuditRecord {
                timestamp,
                actor: "operator".into(),
                source: AuditSource::LocalTui,
                action_id: format!("action-{timestamp}"),
                target_alias: "reflex".into(),
                risk: RiskClass::Normal,
                allowed: true,
                result: "ok".into(),
            });
        }
        let timestamps: Vec<_> = log.newest_first().map(|record| record.timestamp).collect();
        assert_eq!(timestamps, vec![3, 2]);
        assert!(AuditLog::new(0).is_err());
        assert_eq!(AuditRecord::result_summary("F0 01 F7", true), "<redacted>");
        assert_eq!(AuditRecord::result_summary("sent", false), "sent");
    }

    #[test]
    fn authorization_boundary_always_records_redacted_decision() {
        let mut safety = SafetyController::default();
        let mut audit = AuditLog::new(4).expect("audit");
        let decision = safety.authorize_and_record(
            10,
            "operator",
            AuditSource::LocalCli,
            GovernedOperation::HazardousSend,
            "write-1",
            "reflex",
            RiskClass::Destructive,
            ConfirmationClass::Destructive,
            false,
            "F0 01 F7",
            true,
            &mut audit,
        );
        assert_eq!(decision, PolicyDecision::DenyUnsafeMode);
        let record = audit.newest_first().next().expect("record");
        assert!(!record.allowed);
        assert_eq!(record.result, "<redacted>");
        assert_eq!(record.source, AuditSource::LocalCli);
        assert_eq!(record.action_id, "write-1");
    }

    #[test]
    fn activation_summary_counts_terminal_results() {
        let results = vec![
            ("a".into(), ActionResult::Succeeded),
            ("b".into(), ActionResult::SkippedDependency),
            ("c".into(), ActionResult::Cancelled),
            ("d".into(), ActionResult::TimedOut),
            ("e".into(), ActionResult::SentUnverified),
        ];
        assert_eq!(
            ActivationSummary::from_results(&results),
            ActivationSummary {
                succeeded: 1,
                failed: 1,
                skipped: 1,
                cancelled: 1,
                sent_unverified: 1,
            }
        );
        assert_eq!(ActivationSummary::from_results(&results).total(), 5);
        assert!(!ActivationSummary::from_results(&results).is_fully_verified());
        assert!(ActivationSummary::from_results(&[("a".into(), ActionResult::Succeeded)])
            .is_fully_verified());
    }

    #[test]
    fn invalid_dependency_and_duplicate_are_rejected() {
        assert!(ActivationPlan::compile(vec![action("a", false, Some("missing"))]).is_err());
        assert!(ActivationPlan::compile(vec![action("a", false, None), action("a", false, None)])
            .is_err());
    }
}
