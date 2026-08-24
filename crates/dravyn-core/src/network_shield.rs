use crate::profile_runtime;
use dravyn_common::Workspace;
use dravyn_network::{NetworkMode, probe_network};
use dravyn_privacy::NetworkGuardMode;
use dravyn_profile::{Profile, ProfileStore};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const SHIELD_FAILURE_LIMIT: u32 = 3;
pub const SHIELD_PROBE_INTERVAL: Duration = Duration::from_secs(3);
pub const SHIELD_PROBE_TIMEOUT: Duration = Duration::from_millis(900);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkShieldMode {
    Off,
    Monitor,
    Strict,
}

impl NetworkShieldMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Monitor => "monitor",
            Self::Strict => "strict",
        }
    }

    pub fn enforced(self) -> bool {
        self == Self::Strict
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkShieldState {
    Off,
    Standby,
    Monitoring,
    Healthy,
    Degraded,
    Tripped,
}

impl NetworkShieldState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Standby => "standby",
            Self::Monitoring => "monitoring",
            Self::Healthy => "healthy",
            Self::Degraded => "degraded",
            Self::Tripped => "tripped",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NetworkShieldSnapshot {
    pub profile_id: String,
    pub mode: NetworkShieldMode,
    pub state: NetworkShieldState,
    pub endpoint: Option<String>,
    pub running: bool,
    pub policy_version: u32,
    pub last_checked_at: Option<u64>,
    pub consecutive_failures: u32,
    pub failure_limit: u32,
    pub message: String,
}

impl NetworkShieldSnapshot {
    pub fn enforced(&self) -> bool {
        self.mode.enforced()
    }
}

struct ShieldHandle {
    stop: Arc<AtomicBool>,
    snapshot: Arc<Mutex<NetworkShieldSnapshot>>,
}

#[derive(Default)]
pub struct NetworkShieldSupervisor {
    entries: Mutex<HashMap<String, ShieldHandle>>,
}

impl NetworkShieldSupervisor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn snapshot(&self, profile_id: &str) -> Option<NetworkShieldSnapshot> {
        let snapshot = {
            let entries = self.entries.lock().ok()?;
            Arc::clone(&entries.get(profile_id)?.snapshot)
        };
        snapshot.lock().ok().map(|value| value.clone())
    }

    pub fn disarm(&self, profile_id: &str) {
        let handle = self
            .entries
            .lock()
            .ok()
            .and_then(|mut entries| entries.remove(profile_id));
        if let Some(handle) = handle {
            handle.stop.store(true, Ordering::Release);
        }
    }

    pub fn reconcile(
        &self,
        workspace: &Workspace,
        profile: &Profile,
        running: bool,
    ) -> Result<NetworkShieldSnapshot, String> {
        let mode = shield_mode(profile);
        let endpoint = profile.network.endpoint_label();

        if mode == NetworkShieldMode::Off {
            self.disarm(&profile.id);
            return Ok(standby_snapshot(profile, mode, endpoint, running));
        }

        if !running {
            if let Some(existing) = self.snapshot(&profile.id) {
                if existing.state == NetworkShieldState::Tripped {
                    return Ok(existing);
                }
            }
            self.disarm(&profile.id);
            return Ok(standby_snapshot(profile, mode, endpoint, false));
        }

        if let Some(existing) = self.snapshot(&profile.id) {
            let compatible = existing.policy_version == profile.privacy.policy_version
                && existing.mode == mode
                && existing.endpoint == endpoint
                && existing.state != NetworkShieldState::Tripped;
            if compatible {
                return Ok(existing);
            }
        }

        self.arm(workspace.clone(), profile.clone(), mode, endpoint)
    }

    fn arm(
        &self,
        workspace: Workspace,
        profile: Profile,
        mode: NetworkShieldMode,
        endpoint: Option<String>,
    ) -> Result<NetworkShieldSnapshot, String> {
        self.disarm(&profile.id);

        let stop = Arc::new(AtomicBool::new(false));
        let snapshot = Arc::new(Mutex::new(NetworkShieldSnapshot {
            profile_id: profile.id.clone(),
            mode,
            state: NetworkShieldState::Monitoring,
            endpoint,
            running: true,
            policy_version: profile.privacy.policy_version,
            last_checked_at: None,
            consecutive_failures: 0,
            failure_limit: SHIELD_FAILURE_LIMIT,
            message: if mode.enforced() {
                format!(
                    "Strict Network Shield is armed. The profile will be terminated after {SHIELD_FAILURE_LIMIT} consecutive proxy endpoint failures."
                )
            } else {
                "Network Shield is monitoring proxy endpoint health without terminating the profile.".to_owned()
            },
        }));

        let thread_stop = Arc::clone(&stop);
        let thread_snapshot = Arc::clone(&snapshot);
        let thread_profile = profile.clone();
        thread::Builder::new()
            .name(format!("dravyn-network-shield-{}", short_id(&profile.id)))
            .spawn(move || {
                run_supervisor_loop(
                    workspace,
                    thread_profile,
                    mode,
                    thread_stop,
                    thread_snapshot,
                );
            })
            .map_err(|error| format!("failed to start network shield supervisor: {error}"))?;

        let initial = snapshot
            .lock()
            .map_err(|_| "network shield state lock is poisoned".to_owned())?
            .clone();
        self.entries
            .lock()
            .map_err(|_| "network shield registry lock is poisoned".to_owned())?
            .insert(profile.id, ShieldHandle { stop, snapshot });
        Ok(initial)
    }
}

fn run_supervisor_loop(
    workspace: Workspace,
    profile: Profile,
    mode: NetworkShieldMode,
    stop: Arc<AtomicBool>,
    snapshot: Arc<Mutex<NetworkShieldSnapshot>>,
) {
    let store = ProfileStore::new(workspace.clone());

    loop {
        if stop.load(Ordering::Acquire) {
            break;
        }

        match profile_runtime::status(&workspace, &store, &profile) {
            Ok(runtime) if !runtime.running => {
                update_snapshot(&snapshot, |state| {
                    state.running = false;
                    if state.state != NetworkShieldState::Tripped {
                        state.state = NetworkShieldState::Standby;
                        state.message = "Browser runtime stopped; Network Shield is standing by for the next launch.".to_owned();
                    }
                });
                break;
            }
            Ok(_) => {}
            Err(error) => {
                update_snapshot(&snapshot, |state| {
                    state.state = NetworkShieldState::Degraded;
                    state.message = format!("Network Shield could not inspect browser runtime state: {error}");
                });
                sleep_interruptible(&stop, SHIELD_PROBE_INTERVAL);
                continue;
            }
        }

        let probe = probe_network(&profile.network, SHIELD_PROBE_TIMEOUT);
        if stop.load(Ordering::Acquire) {
            break;
        }

        let reachable = probe.reachable == Some(true);
        let mut should_trip = false;
        update_snapshot(&snapshot, |state| {
            state.running = true;
            state.last_checked_at = Some(epoch_seconds());
            if reachable {
                state.consecutive_failures = 0;
                state.state = NetworkShieldState::Healthy;
                state.message = format!(
                    "Proxy endpoint remains reachable ({} ms). This is continuous route-health evidence only; remote IP/DNS/IPv6/WebRTC verification remains separate.",
                    probe.latency_ms.unwrap_or_default()
                );
            } else {
                let (failures, next_state, trip) =
                    failure_transition(mode, state.consecutive_failures);
                state.consecutive_failures = failures;
                state.state = next_state;
                should_trip = trip;
                state.message = if trip {
                    format!(
                        "Strict Network Shield tripped after {failures} consecutive proxy endpoint failures. Dravyn is terminating this profile to avoid continuing with an unhealthy route."
                    )
                } else if mode == NetworkShieldMode::Strict {
                    format!(
                        "Proxy endpoint health check failed ({failures}/{SHIELD_FAILURE_LIMIT}). Dravyn will terminate the profile if failures remain consecutive. {}",
                        probe.message
                    )
                } else {
                    format!("Proxy endpoint health check failed in monitor mode. {}", probe.message)
                };
            }
        });

        if should_trip {
            if stop.load(Ordering::Acquire) {
                break;
            }
            let stop_result = profile_runtime::stop(&workspace, &store, &profile);
            update_snapshot(&snapshot, |state| {
                state.running = false;
                if let Err(error) = stop_result {
                    state.message = format!(
                        "Network Shield tripped, but Dravyn could not confirm browser termination: {error}"
                    );
                }
            });
            break;
        }

        sleep_interruptible(&stop, SHIELD_PROBE_INTERVAL);
    }
}

fn failure_transition(
    mode: NetworkShieldMode,
    current_failures: u32,
) -> (u32, NetworkShieldState, bool) {
    let failures = current_failures.saturating_add(1);
    let trip = mode == NetworkShieldMode::Strict && failures >= SHIELD_FAILURE_LIMIT;
    let state = if trip {
        NetworkShieldState::Tripped
    } else {
        NetworkShieldState::Degraded
    };
    (failures, state, trip)
}

fn shield_mode(profile: &Profile) -> NetworkShieldMode {
    if profile.network.mode != NetworkMode::Proxy {
        return NetworkShieldMode::Off;
    }
    match profile.privacy.network_guard {
        NetworkGuardMode::Off => NetworkShieldMode::Off,
        NetworkGuardMode::Monitor => NetworkShieldMode::Monitor,
        NetworkGuardMode::Strict => NetworkShieldMode::Strict,
    }
}

fn standby_snapshot(
    profile: &Profile,
    mode: NetworkShieldMode,
    endpoint: Option<String>,
    running: bool,
) -> NetworkShieldSnapshot {
    let (state, message) = if mode == NetworkShieldMode::Off {
        (
            NetworkShieldState::Off,
            if profile.network.mode == NetworkMode::Direct {
                "Direct networking is configured; proxy Network Shield monitoring is not required.".to_owned()
            } else {
                "Proxy networking is configured with Network Guard disabled.".to_owned()
            },
        )
    } else {
        (
            NetworkShieldState::Standby,
            if mode.enforced() {
                "Strict Network Shield will arm when this profile launches.".to_owned()
            } else {
                "Network Shield monitoring will arm when this profile launches.".to_owned()
            },
        )
    };
    NetworkShieldSnapshot {
        profile_id: profile.id.clone(),
        mode,
        state,
        endpoint,
        running,
        policy_version: profile.privacy.policy_version,
        last_checked_at: None,
        consecutive_failures: 0,
        failure_limit: SHIELD_FAILURE_LIMIT,
        message,
    }
}

fn update_snapshot(
    snapshot: &Arc<Mutex<NetworkShieldSnapshot>>,
    update: impl FnOnce(&mut NetworkShieldSnapshot),
) {
    if let Ok(mut state) = snapshot.lock() {
        update(&mut state);
    }
}

fn sleep_interruptible(stop: &AtomicBool, duration: Duration) {
    let slices = 6u32;
    let slice_ms = (duration.as_millis() / u128::from(slices)).max(1) as u64;
    for _ in 0..slices {
        if stop.load(Ordering::Acquire) {
            return;
        }
        thread::sleep(Duration::from_millis(slice_ms));
    }
}

fn short_id(id: &str) -> String {
    id.chars().take(12).collect()
}

fn epoch_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_mode_trips_only_after_consecutive_failure_limit() {
        let (first, state, trip) = failure_transition(NetworkShieldMode::Strict, 0);
        assert_eq!(first, 1);
        assert_eq!(state, NetworkShieldState::Degraded);
        assert!(!trip);

        let (third, state, trip) = failure_transition(NetworkShieldMode::Strict, 2);
        assert_eq!(third, SHIELD_FAILURE_LIMIT);
        assert_eq!(state, NetworkShieldState::Tripped);
        assert!(trip);
    }

    #[test]
    fn monitor_mode_never_trips() {
        let (failures, state, trip) = failure_transition(NetworkShieldMode::Monitor, 20);
        assert_eq!(failures, 21);
        assert_eq!(state, NetworkShieldState::Degraded);
        assert!(!trip);
    }
}
