import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import CommercialApp from "./CommercialApp";
import { api } from "./api";
import type { AppStatus, DiagnosticItem, ProfileView } from "./types";

const ONBOARDING_KEY = "dravyn.m7.onboarding.dismissed";
const ACTIVITY_KEY = "dravyn.m7.activity";
const MAX_ACTIVITY = 80;

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

function displayState(value: string) {
  return value.replaceAll("_", " ");
}

export default function ProductionApp() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [profiles, setProfiles] = useState<ProfileView[]>([]);
  const [diagnostics, setDiagnostics] = useState<DiagnosticItem[]>([]);
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
              detail: `${item.verification.core_pass_count}/${4} core network checks currently pass for the active evidence set.`,
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

      previous.current = { profiles: nextProfiles, diagnostics: nextDiagnostics };
      setStatus(nextStatus);
      setProfiles(nextProfiles);
      setDiagnostics(nextDiagnostics);
      setSnapshotReady(true);
    } finally {
      setRefreshing(false);
    }
  }, [appendActivity]);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 8000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const summary = useMemo(() => {
    const diagnosticErrors = diagnostics.filter((item) => item.status === "error").length;
    const diagnosticWarnings = diagnostics.filter((item) => item.status === "warning").length;
    const criticalProfiles = profiles.filter((item) => item.verification.state === "critical").length;
    const driftProfiles = profiles.filter((item) => item.fingerprint.state === "drift").length;
    const unverifiedProfiles = profiles.filter((item) => item.verification.state !== "healthy").length;
    const running = profiles.filter((item) => item.runtime.running).length;

    const tone =
      !status?.chromium_ready || diagnosticErrors > 0 || criticalProfiles > 0
        ? "critical"
        : diagnosticWarnings > 0 || driftProfiles > 0 || unverifiedProfiles > 0
          ? "review"
          : "healthy";

    return {
      tone,
      diagnosticErrors,
      diagnosticWarnings,
      criticalProfiles,
      driftProfiles,
      unverifiedProfiles,
      running,
    };
  }, [diagnostics, profiles, status]);

  function dismissOnboarding() {
    window.localStorage.setItem(ONBOARDING_KEY, "1");
    setOnboardingOpen(false);
  }

  function clearActivity() {
    window.localStorage.removeItem(ACTIVITY_KEY);
    setActivity([]);
  }

  return (
    <div className="m7-shell">
      <CommercialApp />

      <button
        className={`m7-assurance-trigger ${summary.tone}`}
        type="button"
        onClick={() => setPanelOpen(true)}
        aria-label="Open production assurance center"
      >
        <span className="m7-trigger-dot" />
        <span>
          <strong>{summary.tone === "healthy" ? "Healthy" : summary.tone === "critical" ? "Critical" : "Review"}</strong>
          <small>M7 Assurance</small>
        </span>
        {(summary.criticalProfiles + summary.diagnosticErrors) > 0 && (
          <em>{summary.criticalProfiles + summary.diagnosticErrors}</em>
        )}
      </button>

      {panelOpen && (
        <div className="m7-panel-backdrop" onMouseDown={(event) => event.target === event.currentTarget && setPanelOpen(false)}>
          <aside className="m7-assurance-panel">
            <header>
              <div>
                <span>M7 · Production readiness</span>
                <h2>Assurance Center</h2>
                <p>System, profile and verification signals stay separate so failures cannot be hidden by a single score.</p>
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
                <small>{status?.chromium_ready ? "Chromium ready" : "Chromium not ready"} · {profiles.length} profiles · {summary.running} running</small>
              </div>
              <button type="button" onClick={() => void refresh()} disabled={refreshing}>{refreshing ? "Checking…" : "Refresh"}</button>
            </section>

            <section className="m7-kpis">
              <article><span>Critical profiles</span><strong>{summary.criticalProfiles}</strong><small>remote verification</small></article>
              <article><span>Fingerprint drift</span><strong>{summary.driftProfiles}</strong><small>needs baseline review</small></article>
              <article><span>Not verified</span><strong>{summary.unverifiedProfiles}</strong><small>current evidence</small></article>
              <article><span>System issues</span><strong>{summary.diagnosticErrors + summary.diagnosticWarnings}</strong><small>diagnostics</small></article>
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
                  {activity.slice(0, 18).map((item) => (
                    <div className="m7-activity-row" key={item.id}>
                      <span className={`m7-activity-icon ${item.level}`} />
                      <div><strong>{item.title}</strong><small>{item.detail}</small></div>
                      <time>{relativeTime(item.at)}</time>
                    </div>
                  ))}
                </div>
              ) : (
                <div className="m7-empty">State transitions will appear here while Dravyn is running.</div>
              )}
            </section>

            <footer>
              <strong>Production boundary</strong>
              <p>This center reports local application state and operator-recorded remote evidence. It does not claim anonymity, OS-level egress enforcement or an internet-facing Dravyn verification service.</p>
            </footer>
          </aside>
        </div>
      )}

      {onboardingOpen && snapshotReady && (
        <div className="m7-onboarding-backdrop">
          <section className="m7-onboarding">
            <div className="m7-onboarding-brand">D</div>
            <span className="m7-onboarding-eyebrow">M7 · Production readiness</span>
            <h2>Set up Dravyn with an assurance-first workflow.</h2>
            <p className="m7-onboarding-copy">The fastest safe path is to validate Chromium, configure one profile, establish its fingerprint baseline, then verify the remote network view.</p>

            <div className="m7-onboarding-steps">
              <article className={status?.chromium_ready ? "done" : "attention"}>
                <span>{status?.chromium_ready ? "✓" : "1"}</span>
                <div><strong>Runtime readiness</strong><small>{status?.chromium_ready ? "Dravyn Chromium is available." : "Build or configure Chromium before launching profiles."}</small></div>
              </article>
              <article className={profiles.length > 0 ? "done" : "pending"}>
                <span>{profiles.length > 0 ? "✓" : "2"}</span>
                <div><strong>Create a profile</strong><small>Choose browser route and privacy policy together. M7 keeps a last-known-good metadata backup.</small></div>
              </article>
              <article className={profiles.some((item) => item.verification.state === "healthy") ? "done" : "pending"}>
                <span>{profiles.some((item) => item.verification.state === "healthy") ? "✓" : "3"}</span>
                <div><strong>Establish evidence</strong><small>Run the local fingerprint audit, then complete Public IP, WebRTC, DNS and IPv6 verification.</small></div>
              </article>
            </div>

            <div className="m7-onboarding-note">
              <strong>What M7 guarantees</strong>
              <span>Stronger local metadata recovery, versioned profile/privacy state and clearer health visibility. Remote anonymity and OS-level network enforcement still require infrastructure beyond this desktop milestone.</span>
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
