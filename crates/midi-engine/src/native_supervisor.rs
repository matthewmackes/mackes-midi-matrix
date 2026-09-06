//! Native ALSA subscription supervisor with stable hardware identity.
//!
//! Volatile client/port numbers are runtime addresses only. Desired endpoints are retained
//! across disconnects; duplicates and permission failures fail closed.

use crate::{AlsaSequencerAddress, AlsaSequencerLifecycle, EndpointDirection, PhysicalDeviceState};
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::Path;

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
    /// Transport role inferred from the native port capabilities/name.
    pub role: String,
    /// Verified USB vendor ID, if native discovery supplied it.
    pub vendor_id: Option<u16>,
    /// Verified USB product ID, if native discovery supplied it.
    pub product_id: Option<u16>,
    /// Verified USB serial, if native discovery supplied it.
    pub serial: Option<String>,
}

/// Parses a bounded USB uevent/property record into verified identity fields.
///
/// Only complete hexadecimal vendor/product pairs are accepted. A serial is
/// optional because serial-less devices require an explicit operator binding.
#[must_use]
pub fn parse_usb_identity_properties(properties: &str) -> Option<(u16, u16, Option<String>)> {
    let mut vendor = None;
    let mut product = None;
    let mut serial = None;
    for line in properties.lines().take(64) {
        let Some((key, value)) = line.split_once('=') else { continue };
        match key.trim() {
            "ID_VENDOR_ID" => vendor = u16::from_str_radix(value.trim_start_matches("0x"), 16).ok(),
            "ID_MODEL_ID" => product = u16::from_str_radix(value.trim_start_matches("0x"), 16).ok(),
            "ID_SERIAL_SHORT" if !value.trim().is_empty() => serial = Some(value.trim().to_owned()),
            "PRODUCT" => {
                let mut parts = value.split('/');
                vendor = u16::from_str_radix(parts.next().unwrap_or_default(), 16).ok();
                product = u16::from_str_radix(parts.next().unwrap_or_default(), 16).ok();
            }
            _ => {}
        }
    }
    Some((vendor?, product?, serial))
}

/// Reads one bounded udev/sysfs property record for native identity discovery.
#[must_use]
pub fn read_usb_identity_properties(path: &Path) -> Option<(u16, u16, Option<String>)> {
    if std::fs::metadata(path).ok()?.len() > 16 * 1024 {
        return None;
    }
    parse_usb_identity_properties(&std::fs::read_to_string(path).ok()?)
}

/// Resolves a USB identity for an ALSA client name using kernel card metadata.
/// Ambiguous or missing card matches fail closed.
#[cfg(target_os = "linux")]
#[must_use]
pub fn native_usb_identity_for_client_name(
    client_name: &str,
) -> Option<(u16, u16, Option<String>)> {
    let cards = std::fs::read_to_string("/proc/asound/cards").ok()?;
    let needle = normalize_client_name(client_name);
    let mut matching_card = None;
    for block in cards.split("\n ").filter(|block| !block.trim().is_empty()) {
        if normalize_client_name(block).contains(&needle) {
            let card = block.split_whitespace().next()?.parse::<u8>().ok()?;
            if matching_card.replace(card).is_some() {
                return None;
            }
        }
    }
    let card = matching_card?;
    let path = format!("/sys/class/sound/card{card}/device/uevent");
    read_usb_identity_properties(Path::new(&path))
}

/// Returns verified USB fields for one native client, or explicit unknown fields.
#[must_use]
#[cfg(target_os = "linux")]
pub fn usb_fields_for_client(client_name: &str) -> (Option<u16>, Option<u16>, Option<String>) {
    native_usb_identity_for_client_name(client_name)
        .map_or((None, None, None), |(vendor, product, serial)| {
            (Some(vendor), Some(product), serial)
        })
}

/// Returns explicit unknown USB fields on platforms without ALSA card metadata.
#[cfg(not(target_os = "linux"))]
#[must_use]
pub const fn usb_fields_for_client(
    _client_name: &str,
) -> (Option<u16>, Option<u16>, Option<String>) {
    (None, None, None)
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
            role: "midi".into(),
            vendor_id: None,
            product_id: None,
            serial: None,
        }
    }

    /// Builds an identity from native ALSA metadata while keeping runtime addresses out.
    #[cfg(feature = "alsa-seq-backend")]
    #[must_use]
    pub fn from_alsa_port(port: &crate::AlsaSequencerPort) -> Self {
        let direction =
            if port.readable { EndpointDirection::Input } else { EndpointDirection::Output };
        let role = if port.port_name.to_ascii_lowercase().contains("hui") { "hui" } else { "midi" };
        Self {
            client_name: normalize_client_name(&port.client_name),
            port_name: port.port_name.clone(),
            port_index: port.address.port,
            direction,
            role: role.into(),
            vendor_id: port.vendor_id,
            product_id: port.product_id,
            serial: port.serial.clone(),
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

    #[test]
    fn usb_properties_require_complete_vendor_product_identity() {
        assert_eq!(
            parse_usb_identity_properties(
                "ID_VENDOR_ID=1235\nID_MODEL_ID=0061\nID_SERIAL_SHORT=abc"
            ),
            Some((0x1235, 0x0061, Some("abc".into())))
        );
        assert_eq!(parse_usb_identity_properties("ID_VENDOR_ID=1235\nID_SERIAL_SHORT=abc"), None);
    }

    #[test]
    fn usb_properties_parse_kernel_product_tuple() {
        assert_eq!(
            parse_usb_identity_properties("PRODUCT=1235/0061/0\n"),
            Some((0x1235, 0x0061, None))
        );
    }

    #[test]
    fn usb_properties_keep_serialless_identity_explicit() {
        assert_eq!(
            parse_usb_identity_properties("ID_VENDOR_ID=0763\nID_MODEL_ID=1021\n"),
            Some((0x0763, 0x1021, None))
        );
    }

    #[test]
    fn usb_property_reader_is_bounded_and_reuses_parser() {
        let path =
            std::env::temp_dir().join(format!("mackes-usb-properties-{}", std::process::id()));
        std::fs::write(&path, "ID_VENDOR_ID=1235\nID_MODEL_ID=0061\n").expect("write fixture");
        assert_eq!(read_usb_identity_properties(&path), Some((0x1235, 0x0061, None)));
        std::fs::write(&path, vec![b'x'; 16 * 1024 + 1]).expect("write oversized fixture");
        assert_eq!(read_usb_identity_properties(&path), None);
        let _ = std::fs::remove_file(path);
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
