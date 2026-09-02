//! Native ALSA subscription supervisor with stable hardware identity.
//!
//! Volatile client/port numbers are runtime addresses only. Desired endpoints are retained
//! across disconnects; duplicates and permission failures fail closed.

use crate::{AlsaSequencerAddress, AlsaSequencerLifecycle, EndpointDirection, PhysicalDeviceState};
use std::collections::{BTreeMap, HashMap, VecDeque};

/// Maximum announcement records processed in one reconciliation.
pub const MAX_NATIVE_ANNOUNCEMENTS: usize = 64;

/// Stable hardware identity independent of volatile ALSA client numbers.
#[derive(Clone, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub struct NativeHardwareIdentity {
    /// Normalized ALSA client display name.
    pub client_name: String,
    /// Exact ALSA port display name.
    pub port_name: String,
    /// Configured port index used for disambiguation.
    pub port_index: u8,
    /// Endpoint direction.
    pub direction: EndpointDirection,
}

impl NativeHardwareIdentity {
    /// Builds an identity from display metadata.
    #[must_use]
    pub fn new(
        client_name: impl AsRef<str>,
        port_name: impl AsRef<str>,
        port_index: u8,
        direction: EndpointDirection,
    ) -> Self {
        Self {
            client_name: normalize_client_name(client_name.as_ref()),
            port_name: port_name.as_ref().to_owned(),
            port_index,
            direction,
        }
    }
}

/// One fake or observed native port announcement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativePortAnnouncement {
    /// Lifecycle reason.
    pub lifecycle: AlsaSequencerLifecycle,
    /// Volatile runtime address.
    pub address: AlsaSequencerAddress,
    /// Hardware identity if known for this announcement.
    pub identity: Option<NativeHardwareIdentity>,
    /// When true, subscription restore is denied.
    pub permission_denied: bool,
}

/// Published supervisor transition for daemon/LED consumers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeIdentityTransition {
    /// Stable application endpoint ID.
    pub stable_id: String,
    /// Hardware identity that produced the transition.
    pub identity: NativeHardwareIdentity,
    /// Current visibility state.
    pub state: PhysicalDeviceState,
    /// Current volatile address when visible and unambiguous.
    pub address: Option<AlsaSequencerAddress>,
    /// True when the matching output returned and LEDs should be replayed.
    pub led_resync: bool,
    /// Visible fail-closed reason, if any.
    pub failure: Option<&'static str>,
}

/// Desired native endpoint retained across reconnects and daemon restarts.
#[derive(Clone, Debug, Eq, PartialEq)]
struct DesiredEndpoint {
    stable_id: String,
    identity: NativeHardwareIdentity,
    address: Option<AlsaSequencerAddress>,
    state: PhysicalDeviceState,
}

/// Reconciles desired subscriptions from announcement streams.
#[derive(Debug)]
pub struct NativeAlsaSupervisor {
    desired: BTreeMap<NativeHardwareIdentity, DesiredEndpoint>,
    live: HashMap<AlsaSequencerAddress, NativeHardwareIdentity>,
    pending: VecDeque<NativePortAnnouncement>,
    generation: u64,
}

impl NativeAlsaSupervisor {
    /// Creates an empty supervisor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            desired: BTreeMap::new(),
            live: HashMap::new(),
            pending: VecDeque::new(),
            generation: 0,
        }
    }

    /// Restores desired identities after a daemon restart without using volatile numbers.
    pub fn remember(&mut self, stable_id: impl Into<String>, identity: NativeHardwareIdentity) {
        self.desired.insert(
            identity.clone(),
            DesiredEndpoint {
                stable_id: stable_id.into(),
                identity,
                address: None,
                state: PhysicalDeviceState::Offline,
            },
        );
    }

    /// Queues one announcement without sleeping or routing.
    pub fn ingest(&mut self, announcement: NativePortAnnouncement) {
        if self.pending.len() >= MAX_NATIVE_ANNOUNCEMENTS {
            self.pending.pop_front();
        }
        self.pending.push_back(announcement);
    }

    /// Reconciles the queued storm into coalesced identity transitions.
    pub fn reconcile(&mut self) -> Vec<NativeIdentityTransition> {
        let mut latest: HashMap<AlsaSequencerAddress, NativePortAnnouncement> = HashMap::new();
        while let Some(announcement) = self.pending.pop_front() {
            latest.insert(announcement.address, announcement);
        }
        let mut batch: Vec<_> = latest.into_values().collect();
        batch.sort_by_key(|announcement| match announcement.lifecycle {
            AlsaSequencerLifecycle::Exited | AlsaSequencerLifecycle::Unsubscribed => 0_u8,
            _ => 1,
        });
        let mut transitions = Vec::new();
        for announcement in batch {
            if let Some(transition) = self.apply(&announcement) {
                transitions.push(transition);
            }
        }
        transitions.sort_by(|left, right| left.identity.cmp(&right.identity));
        self.generation = self.generation.saturating_add(1);
        transitions
    }

    fn apply(&mut self, announcement: &NativePortAnnouncement) -> Option<NativeIdentityTransition> {
        if announcement.permission_denied {
            return Some(self.fail_closed(announcement, "native ALSA permission denied"));
        }
        match announcement.lifecycle {
            AlsaSequencerLifecycle::Started
            | AlsaSequencerLifecycle::Changed
            | AlsaSequencerLifecycle::Subscribed => self.apply_visible(announcement),
            AlsaSequencerLifecycle::Exited | AlsaSequencerLifecycle::Unsubscribed => {
                self.apply_exit(announcement)
            }
        }
    }

    fn apply_visible(
        &mut self,
        announcement: &NativePortAnnouncement,
    ) -> Option<NativeIdentityTransition> {
        let identity = announcement.identity.clone()?;
        let desired = self.desired.get_mut(&identity)?;
        if let Some(existing) = self.live.get(&announcement.address) {
            if existing != &identity {
                return Some(self.fail_closed(announcement, "stale native ALSA announcement"));
            }
        }
        let duplicates = self
            .live
            .iter()
            .filter(|(address, live)| **live == identity && **address != announcement.address)
            .count();
        if duplicates > 0 {
            desired.address = None;
            desired.state = PhysicalDeviceState::Ambiguous;
            return Some(NativeIdentityTransition {
                stable_id: desired.stable_id.clone(),
                identity,
                state: PhysicalDeviceState::Ambiguous,
                address: None,
                led_resync: false,
                failure: Some("duplicate native ALSA identity"),
            });
        }
        let previous = desired.address;
        desired.address = Some(announcement.address);
        desired.state = PhysicalDeviceState::Connected;
        let led_resync = identity.direction == EndpointDirection::Output
            && previous != Some(announcement.address);
        self.live.insert(announcement.address, identity.clone());
        if let Some(previous) = previous {
            if previous != announcement.address {
                self.live.remove(&previous);
            }
        }
        Some(NativeIdentityTransition {
            stable_id: desired.stable_id.clone(),
            identity,
            state: PhysicalDeviceState::Connected,
            address: Some(announcement.address),
            led_resync,
            failure: None,
        })
    }

    fn apply_exit(
        &mut self,
        announcement: &NativePortAnnouncement,
    ) -> Option<NativeIdentityTransition> {
        let identity = announcement
            .identity
            .clone()
            .or_else(|| self.live.get(&announcement.address).cloned())?;
        let desired = self.desired.get_mut(&identity)?;
        if desired.address.is_some_and(|address| address != announcement.address) {
            return None;
        }
        self.live.remove(&announcement.address);
        desired.address = None;
        desired.state = PhysicalDeviceState::Offline;
        Some(NativeIdentityTransition {
            stable_id: desired.stable_id.clone(),
            identity,
            state: PhysicalDeviceState::Offline,
            address: None,
            led_resync: false,
            failure: None,
        })
    }

    fn fail_closed(
        &mut self,
        announcement: &NativePortAnnouncement,
        failure: &'static str,
    ) -> NativeIdentityTransition {
        let identity = announcement.identity.clone().unwrap_or_else(|| {
            NativeHardwareIdentity::new("unknown", "unknown", 0, EndpointDirection::Input)
        });
        if let Some(desired) = self.desired.get_mut(&identity) {
            desired.address = None;
            desired.state = PhysicalDeviceState::Ambiguous;
            NativeIdentityTransition {
                stable_id: desired.stable_id.clone(),
                identity,
                state: PhysicalDeviceState::Ambiguous,
                address: None,
                led_resync: false,
                failure: Some(failure),
            }
        } else {
            NativeIdentityTransition {
                stable_id: String::new(),
                identity,
                state: PhysicalDeviceState::Unknown,
                address: None,
                led_resync: false,
                failure: Some(failure),
            }
        }
    }
}

impl Default for NativeAlsaSupervisor {
    fn default() -> Self {
        Self::new()
    }
}

fn normalize_client_name(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk2(direction: EndpointDirection) -> NativeHardwareIdentity {
        NativeHardwareIdentity::new("Launch Control XL MK2", "MIDI", 0, direction)
    }

    fn announce(
        lifecycle: AlsaSequencerLifecycle,
        client: u8,
        port: u8,
        identity: NativeHardwareIdentity,
    ) -> NativePortAnnouncement {
        NativePortAnnouncement {
            lifecycle,
            address: AlsaSequencerAddress::new(client, port),
            identity: Some(identity),
            permission_denied: false,
        }
    }

    #[test]
    fn changed_client_number_restores_stable_identity() {
        let mut supervisor = NativeAlsaSupervisor::new();
        supervisor.remember("lcxl-in", mk2(EndpointDirection::Input));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        assert_eq!(supervisor.reconcile()[0].address.unwrap().client, 24);
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Exited,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            31,
            0,
            mk2(EndpointDirection::Input),
        ));
        let transitions = supervisor.reconcile();
        let connected = transitions
            .iter()
            .find(|transition| transition.state == PhysicalDeviceState::Connected)
            .expect("reconnected");
        assert_eq!(connected.stable_id, "lcxl-in");
        assert_eq!(connected.address.unwrap().client, 31);
        assert!(!connected.led_resync);
    }

    #[test]
    fn removal_during_traffic_keeps_desired_endpoint_offline() {
        let mut supervisor = NativeAlsaSupervisor::new();
        supervisor.remember("lcxl-in", mk2(EndpointDirection::Input));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        supervisor.reconcile();
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Exited,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        let transition = &supervisor.reconcile()[0];
        assert_eq!(transition.state, PhysicalDeviceState::Offline);
        assert!(supervisor.desired.contains_key(&mk2(EndpointDirection::Input)));
    }

    #[test]
    fn duplicate_launch_controls_fail_closed() {
        let mut supervisor = NativeAlsaSupervisor::new();
        supervisor.remember("lcxl-in", mk2(EndpointDirection::Input));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        supervisor.reconcile();
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            25,
            0,
            mk2(EndpointDirection::Input),
        ));
        let transition = &supervisor.reconcile()[0];
        assert_eq!(transition.state, PhysicalDeviceState::Ambiguous);
        assert_eq!(transition.failure, Some("duplicate native ALSA identity"));
        assert!(transition.address.is_none());
    }

    #[test]
    fn asymmetric_input_return_does_not_resync_leds() {
        let mut supervisor = NativeAlsaSupervisor::new();
        supervisor.remember("lcxl-in", mk2(EndpointDirection::Input));
        supervisor.remember("lcxl-out", mk2(EndpointDirection::Output));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        let transitions = supervisor.reconcile();
        assert!(transitions.iter().all(|transition| !transition.led_resync));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            24,
            1,
            mk2(EndpointDirection::Output),
        ));
        let output = supervisor.reconcile();
        assert_eq!(output.len(), 1);
        assert!(output[0].led_resync);
        assert_eq!(output[0].identity.direction, EndpointDirection::Output);
    }

    #[test]
    fn event_storm_coalesces_to_one_transition() {
        let mut supervisor = NativeAlsaSupervisor::new();
        supervisor.remember("lcxl-in", mk2(EndpointDirection::Input));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Changed,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Subscribed,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        assert_eq!(supervisor.reconcile().len(), 1);
    }

    #[test]
    fn stale_exit_for_previous_address_is_ignored() {
        let mut supervisor = NativeAlsaSupervisor::new();
        supervisor.remember("lcxl-in", mk2(EndpointDirection::Input));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            31,
            0,
            mk2(EndpointDirection::Input),
        ));
        supervisor.reconcile();
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Exited,
            24,
            0,
            mk2(EndpointDirection::Input),
        ));
        assert!(supervisor.reconcile().is_empty());
    }

    #[test]
    fn daemon_restart_restores_from_identity_not_volatile_address() {
        let mut supervisor = NativeAlsaSupervisor::new();
        supervisor.remember("lcxl-in", mk2(EndpointDirection::Input));
        assert!(supervisor
            .desired
            .get(&mk2(EndpointDirection::Input))
            .is_some_and(|endpoint| endpoint.address.is_none()));
        supervisor.ingest(announce(
            AlsaSequencerLifecycle::Started,
            40,
            0,
            mk2(EndpointDirection::Input),
        ));
        assert_eq!(supervisor.reconcile()[0].address.unwrap().client, 40);
    }

    #[test]
    fn permission_failure_is_visible_and_fail_closed() {
        let mut supervisor = NativeAlsaSupervisor::new();
        supervisor.remember("lcxl-in", mk2(EndpointDirection::Input));
        supervisor.ingest(NativePortAnnouncement {
            lifecycle: AlsaSequencerLifecycle::Started,
            address: AlsaSequencerAddress::new(24, 0),
            identity: Some(mk2(EndpointDirection::Input)),
            permission_denied: true,
        });
        let transition = &supervisor.reconcile()[0];
        assert_eq!(transition.failure, Some("native ALSA permission denied"));
        assert_eq!(transition.state, PhysicalDeviceState::Ambiguous);
    }
}

#[cfg(all(test, feature = "alsa-seq-backend"))]
mod live_tests {
    #[test]
    #[ignore = "physical USB reconnect qualification is tracked by W088"]
    fn physical_alsa_reconnect_restores_stable_identity() {
        panic!("run the W088 Mk2 USB reconnect walkthrough with changed ALSA client numbers");
    }
}
