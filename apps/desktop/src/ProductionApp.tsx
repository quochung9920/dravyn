import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import CommercialApp from "./CommercialApp";
import { api } from "./api";
import type {
  AppStatus,
  DiagnosticItem,
  NetworkShieldStatus,
  ProfileView,
} from "./types";

const ONBOARDING_KEY = "dravyn.m8.onboarding.dismissed";
const ACTIVITY_KEY = "dravyn.m8.activity";
const MAX_ACTIVITY = 100;

type ActivityLevel = "info" | "warning" | "critical" | "success";

type ActivityEvent = {
  id: string;
  at: number;
  level: ActivityLevel;
  title: string;
  detail: string;
};

type Snapshot = {
  profiles: ProfileView[];
  diagnostics: DiagnosticItem[];
  shields: Record<string, NetworkShieldStatus>;
};

function readActivity(): ActivityEvent[] {
  try {
    const value = window.localStorage.getItem(ACTIVITY_KEY);
    if (!value) return [];
    const parsed = JSON.parse(value) as ActivityEvent[];
    return Array.isArray(parsed) ? parsed.slice(0, MAX_ACTIVITY) : [];
  } catch {
    return [];
  }
}

function eventId() {
  return `${Date.now()}-${Math.random().toString(16).slice(2)}`;
}

function relativeTime(timestamp: number) {
  const seconds = Math.max(0, Math.floor((Date.now() - timestamp) / 1000));
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

function relativeEpoch(timestamp: number | null) {
  return timestamp ? relativeTime(timestamp * 1000) : "Not checked";
}

function displayState(value: string) {
  return value.replaceAll("_", " ");
}

function shieldTone(state: NetworkShieldStatus["state"]): ActivityLevel {
  if (state === "tripped") return "critical";
  if (state === "degraded") return "warning";
  if (state === "healthy") return "success";
  return "info";
}

export default function ProductionApp() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [profiles, setProfiles] = useState<ProfileView[]>([]);
  const [diagnostics, setDiagnostics] = useState<DiagnosticItem[]>([]);
  const [shields, setShields] = useState<Record<string, NetworkShieldStatus>>({});
  const [activity, setActivity] = useState<ActivityEvent[]>(readActivity);
  const [panelOpen, setPanelOpen] = useState(false);
  const [onboardingOpen, setOnboardingOpen] = useState(
    () => window.localStorage.getItem(ONBOARDING_KEY) !== "1",
  );
  const [refreshing, setRefreshing] = useState(false);
  const [snapshotReady, setSnapshotReady] = useState(false);
  const previous = useRef<Snapshot | null>(null);

  const appendActivity = useCallback((events: ActivityEvent[]) => {
    if (!events.length) return;
    setActivity((current) => {
      const next = [...events, ...current].slice(0, MAX_ACTIVITY);
      window.localStorage.setItem(ACTIVITY_KEY, JSON.stringify(next));
      return next;
    });
  }, []);

  const refresh = useCallback(async () => {
    setRefreshing(true);
    try {
      const [nextStatus, nextProfiles, nextDiagnostics] = await Promise.all([
        api.appStatus(),
        api.listProfiles(),
        api.systemDiagnostics(),
      ]);

      const shieldEntries = await Promise.all(
        nextProfiles.map(async (item) => {
          try {
            const shield = await api.networkShieldStatus(item.profile.id);
            return [item.profile.id, shield] as const;
          } catch {
            return null;
          }
        }),
      );
      const nextShields = Object.fromEntries(
        shieldEntries.filter(
          (entry): entry is readonly [string, NetworkShieldStatus] => entry !== null,
        ),
      );

      const before = previous.current;
      if (before) {
        const events: ActivityEvent[] = [];
        const beforeProfiles = new Map(before.profiles.map((item) => [item.profile.id, item]));

        for (const item of nextProfiles) {
          const prior = beforeProfiles.get(item.profile.id);
          if (!prior) {
            events.push({
              id: eventId(),
              at: Date.now(),
              level: "success",
              title: `Profile created · ${item.profile.name}`,
              detail: `Profile schema v${item.profile.schema_version ?? 1}, privacy policy v${item.profile.privacy.policy_version ?? 1}.`,
            });
            continue;
          }
          if (prior.runtime.running !== item.runtime.running) {
            events.push({
              id: eventId(),
              at: Date.now(),
              level: item.runtime.running ? "success" : "info",
              title: `${item.profile.name} ${item.runtime.running ? "started" : "stopped"}`,
              detail: item.runtime.running
                ? "Chromium runtime is active for this isolated profile."
                : "Chromium runtime is no longer active for this profile.",
            });
          }
          if (prior.verification.state !== item.verification.state) {
            events.push({
              id: eventId(),
              at: Date.now(),
              level:
                item.verification.state === "critical"
                  ? "critical"
                  : item.verification.state === "healthy"
                    ? "success"
                    : "warning",
              title: `${item.profile.name} verification → ${displayState(item.verification.state)}`,
              detail: `${item.verification.core_pass_count}/4 core network checks currently pass for the active policy evidence set.`,
            });
          }
          if (prior.verification_fresh !== item.verification_fresh) {
            events.push({
              id: eventId(),
              at: Date.now(),
              level: item.verification_fresh ? "success" : "warning",
              title: `${item.profile.name} verification ${item.verification_fresh ? "is fresh" : "expired"}`,
              detail: item.verification_fresh
                ? "The latest verification evidence is inside this profile's configured freshness window."
                : "Remote verification should be repeated before this profile is treated as healthy.",
            });
          }
          if (prior.fingerprint.state !== item.fingerprint.state) {
            events.push({
              id: eventId(),
              at: Date.now(),
              level:
                item.fingerprint.state === "drift"
                  ? "warning"
                  : item.fingerprint.state === "stable"
                    ? "success"
                    : "info",
              title: `${item.profile.name} fingerprint → ${displayState(item.fingerprint.state)}`,
              detail: `${item.fingerprint.drift_count} drift and ${item.fingerprint.issue_count} review item(s).`,
            });
          }

          const priorShield = before.shields[item.profile.id];
          const nextShield = nextShields[item.profile.id];
          if (priorShield && nextShield && priorShield.state !== nextShield.state) {
            events.push({
              id: eventId(),
              at: Date.now(),
              level: shieldTone(nextShield.state),
              title: `${item.profile.name} Network Shield → ${displayState(nextShield.state)}`,
              detail: nextShield.message,
            });
          }
        }

        const beforeDiagnostics = new Map(before.diagnostics.map((item) => [item.id, item.status]));
        for (const item of nextDiagnostics) {
          if (beforeDiagnostics.get(item.id) !== item.status && item.status !== "ok") {
            events.push({
              id: eventId(),
              at: Date.now(),
              level: item.status === "error" ? "critical" : "warning",
              title: `${item.label} needs attention`,
              detail: item.detail,
            });
          }
        }
        appendActivity(events);
      }

      previous.current = {
        profiles: nextProfiles,
        diagnostics: nextDiagnostics,
        shields: nextShields,
      };
      setStatus(nextStatus);
      setProfiles(nextProfiles);
      setDiagnostics(nextDiagnostics);
      setShields(nextShields);
      setSnapshotReady(true);
    } finally {
      setRefreshing(false);
    }
  }, [appendActivity]);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 6000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const summary = useMemo(() => {
    const diagnosticErrors = diagnostics.filter((item) => item.status === "error").length;
    const diagnosticWarnings = diagnostics.filter((item) => item.status === "warning").length;
    const criticalProfiles = profiles.filter((item) => item.verification.state === "critical").length;
    const driftProfiles = profiles.filter((item) => item.fingerprint.state === "drift").length;
    const verificationDue = profiles.filter(
      (item) => item.verification.state !== "healthy" || !item.verification_fresh,
    ).length;
    const running = profiles.filter((item) => item.runtime.running).length;
    const shieldValues = Object.values(shields);
    const shieldTripped = shieldValues.filter((item) => item.state === "tripped").length;
    const shieldDegraded = shieldValues.filter((item) => item.state === "degraded").length;
    const shieldHealthy = shieldValues.filter(
      (item) => item.state === "healthy" || item.state === "monitoring",
    ).length;

    const tone =
      !status?.chromium_ready || diagnosticErrors > 0 || criticalProfiles > 0 || shieldTripped > 0
        ? "critical"
        : diagnosticWarnings > 0 || driftProfiles > 0 || verificationDue > 0 || shieldDegraded > 0
          ? "review"
          : "healthy";

    return {
      tone,
      diagnosticErrors,
      diagnosticWarnings,
      criticalProfiles,
      driftProfiles,
      verificationDue,
      running,
      shieldTripped,
      shieldDegraded,
      shieldHealthy,
    };
  }, [diagnostics, profiles, shields, status]);

  const shieldRows = useMemo(
    () =>
      profiles
        .filter((item) => item.profile.network.mode === "proxy")
        .map((item) => ({ item, shield: shields[item.profile.id] ?? null })),
    [profiles, shields],
  );

  function dismissOnboarding() {
    window.localStorage.setItem(ONBOARDING_KEY, "1");
    setOnboardingOpen(false);
  }

  function clearActivity() {
    window.localStorage.removeItem(ACTIVITY_KEY);
    setActivity([]);
  }

  return (
    <div className="m7-shell m8-shell">
      <CommercialApp />

      <button
        className={`m7-assurance-trigger ${summary.tone}`}
        type="button"
        onClick={() => setPanelOpen(true)}
        aria-label="Open continuous assurance center"
      >
        <span className="m7-trigger-dot" />
        <span>
          <strong>{summary.tone === "healthy" ? "Healthy" : summary.tone === "critical" ? "Critical" : "Review"}</strong>
          <small>M8 Continuous Assurance</small>
        </span>
        {(summary.criticalProfiles + summary.diagnosticErrors + summary.shieldTripped) > 0 && (
          <em>{summary.criticalProfiles + summary.diagnosticErrors + summary.shieldTripped}</em>
        )}
      </button>

      {panelOpen && (
        <div className="m7-panel-backdrop" onMouseDown={(event) => event.target === event.currentTarget && setPanelOpen(false)}>
          <aside className="m7-assurance-panel m8-assurance-panel">
            <header>
              <div>
                <span>M8 · Network Shield & continuous assurance</span>
                <h2>Assurance Center</h2>
                <p>Runtime route health, verification freshness, fingerprint drift and system readiness stay visible as separate evidence.</p>
              </div>
              <button type="button" onClick={() => setPanelOpen(false)}>×</button>
            </header>

            <section className="m7-health-banner">
              <div className={`m7-health-mark ${summary.tone}`}>
                {summary.tone === "healthy" ? "✓" : summary.tone === "critical" ? "!" : "◇"}
              </div>
              <div>
                <span>Current state</span>
                <strong>{summary.tone === "healthy" ? "Ready" : summary.tone === "critical" ? "Action required" : "Review recommended"}</strong>
                <small>{status?.chromium_ready ? "Chromium ready" : "Chromium not ready"} · {profiles.length} profiles · {summary.running} running · {summary.shieldHealthy} shield active</small>
              </div>
              <button type="button" onClick={() => void refresh()} disabled={refreshing}>{refreshing ? "Checking…" : "Refresh"}</button>
            </section>

            <section className="m7-kpis">
              <article><span>Critical profiles</span><strong>{summary.criticalProfiles}</strong><small>remote verification</small></article>
              <article><span>Shield alerts</span><strong>{summary.shieldTripped + summary.shieldDegraded}</strong><small>{summary.shieldTripped} tripped · {summary.shieldDegraded} degraded</small></article>
              <article><span>Fingerprint drift</span><strong>{summary.driftProfiles}</strong><small>needs baseline review</small></article>
              <article><span>Verification due</span><strong>{summary.verificationDue}</strong><small>missing, review or expired</small></article>
            </section>

            <section className="m7-panel-section">
              <div className="m7-section-title"><div><span>Continuous route health</span><h3>Network Shield</h3></div></div>
              {shieldRows.length ? (
                <div className="m8-shield-list">
                  {shieldRows.map(({ item, shield }) => (
                    <div className="m8-shield-row" key={item.profile.id}>
                      <span className={`m8-shield-icon ${shield?.state ?? "standby"}`} />
                      <div className="m8-shield-main">
                        <strong>{item.profile.name}</strong>
                        <small>{shield?.message ?? "Shield state is loading."}</small>
                        <div className="m8-shield-meta">
                          <span>{shield?.mode ?? item.profile.privacy.network_guard}</span>
                          <span>{shield?.endpoint ?? "Proxy endpoint"}</span>
                          <span>checked {relativeEpoch(shield?.last_checked_at ?? null)}</span>
                          {shield && shield.consecutive_failures > 0 && (
                            <span className="warning">failures {shield.consecutive_failures}/{shield.failure_limit}</span>
                          )}
                        </div>
                      </div>
                      <em className={`m8-shield-badge ${shield?.state ?? "standby"}`}>
                        {displayState(shield?.state ?? "standby")}
                      </em>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="m7-empty">No proxy profiles require Network Shield monitoring yet.</div>
              )}
              <p className="m8-boundary-note">Strict mode terminates a running profile after three consecutive proxy endpoint failures while Dravyn Desktop is alive. It does not replace an OS firewall or prove remote IP/DNS/IPv6/WebRTC behavior.</p>
            </section>

            <section className="m7-panel-section">
              <div className="m7-section-title"><div><span>Readiness</span><h3>System checks</h3></div></div>
              <div className="m7-diagnostic-list">
                {diagnostics.map((item) => (
                  <div className="m7-diagnostic-row" key={item.id}>
                    <span className={`m7-status-dot ${item.status}`} />
                    <div><strong>{item.label}</strong><small>{item.detail}</small></div>
                    <em>{item.status}</em>
                  </div>
                ))}
              </div>
            </section>

            <section className="m7-panel-section">
              <div className="m7-section-title">
                <div><span>Local audit trail</span><h3>Recent state changes</h3></div>
                {activity.length > 0 && <button type="button" onClick={clearActivity}>Clear</button>}
              </div>
              {activity.length ? (
                <div className="m7-activity-list">
                  {activity.slice(0, 24).map((item) => (
                    <div className="m7-activity-row" key={item.id}>
                      <span className={`m7-activity-icon ${item.level}`} />
                      <div><strong>{item.title}</strong><small>{item.detail}</small></div>
                      <time>{relativeTime(item.at)}</time>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="m7-empty">Runtime, shield, fingerprint and verification state changes will appear here.</div>
              )}
            </section>

            <footer>
              <strong>Assurance boundary</strong>
              <p>M8 adds a continuous process-level proxy kill-switch and fresher health semantics. It still does not claim anonymity, OS-level egress firewalling or an internet-facing Dravyn verification service.</p>
            </footer>
          </aside>
        </div>
      )}

      {onboardingOpen && snapshotReady && (
        <div className="m7-onboarding-backdrop">
          <section className="m7-onboarding m8-onboarding">
            <div className="m7-onboarding-brand">D</div>
            <span className="m7-onboarding-eyebrow">M8 · Continuous assurance</span>
            <h2>Set up a profile that stays observable while it runs.</h2>
            <p className="m7-onboarding-copy">Validate Chromium, configure the route and privacy policy, let Network Shield watch strict proxy health, then establish fingerprint and external verification evidence.</p>

            <div className="m7-onboarding-steps">
              <article className={status?.chromium_ready ? "done" : "attention"}>
                <span>{status?.chromium_ready ? "✓" : "1"}</span>
                <div><strong>Runtime readiness</strong><small>{status?.chromium_ready ? "Dravyn Chromium is available." : "Build or configure Chromium before launching profiles."}</small></div>
              </article>
              <article className={profiles.length > 0 ? "done" : "pending"}>
                <span>{profiles.length > 0 ? "✓" : "2"}</span>
                <div><strong>Create a profile</strong><small>Choose browser route and privacy policy together. Profile metadata remains versioned and recoverable.</small></div>
              </article>
              <article className={Object.values(shields).some((item) => item.state === "healthy") ? "done" : "pending"}>
                <span>{Object.values(shields).some((item) => item.state === "healthy") ? "✓" : "3"}</span>
                <div><strong>Arm route monitoring</strong><small>Monitor/Strict proxy profiles are watched continuously while the desktop app is running; Strict mode adds the three-failure kill-switch.</small></div>
              </article>
              <article className={profiles.some((item) => item.verification.state === "healthy" && item.verification_fresh) ? "done" : "pending"}>
                <span>{profiles.some((item) => item.verification.state === "healthy" && item.verification_fresh) ? "✓" : "4"}</span>
                <div><strong>Establish evidence</strong><small>Run the local fingerprint audit, then complete fresh Public IP, WebRTC, DNS and IPv6 verification.</small></div>
              </article>
            </div>

            <div className="m7-onboarding-note">
              <strong>What M8 adds</strong>
              <span>Bounded preflight latency, continuous proxy endpoint health monitoring, a strict process kill-switch, verification freshness awareness and an operator-visible shield timeline. Remote leak proof and OS firewalling remain separate future infrastructure.</span>
            </div>

            <div className="m7-onboarding-actions">
              <button type="button" className="secondary" onClick={dismissOnboarding}>Skip for now</button>
              <button type="button" className="primary" onClick={dismissOnboarding}>Open Dravyn</button>
            </div>
          </section>
        </div>
      )}
    </div>
  );
}
