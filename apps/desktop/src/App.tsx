import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import type {
  AppStatus,
  DiagnosticItem,
  ExternalVerificationTest,
  FingerprintHistoryEntry,
  FingerprintSnapshot,
  FingerprintState,
  NetworkGuardMode,
  NetworkMode,
  NetworkProbe,
  PrivacyPolicy,
  PrivacyPreset,
  PrivacyStatus,
  Profile,
  ProfileDraft,
  ProfileView,
  ProxyScheme,
  WebRtcPolicy,
} from "./types";
import "./fingerprint.css";
import "./privacy.css";

type Page =
  | "dashboard"
  | "profiles"
  | "fingerprints"
  | "privacy"
  | "network"
  | "diagnostics"
  | "settings";
type SortMode = "updated" | "name" | "running" | "fingerprint";

const navItems: Array<{ id: Page; label: string; icon: string }> = [
  { id: "dashboard", label: "Dashboard", icon: "⌂" },
  { id: "profiles", label: "Profiles", icon: "◎" },
  { id: "fingerprints", label: "Fingerprints", icon: "◇" },
  { id: "privacy", label: "Privacy", icon: "◈" },
  { id: "network", label: "Network", icon: "⌁" },
  { id: "diagnostics", label: "Diagnostics", icon: "▤" },
  { id: "settings", label: "Settings", icon: "⚙" },
];

const pageCopy: Record<Page, { eyebrow: string; title: string; subtitle: string }> = {
  dashboard: {
    eyebrow: "M5 · Privacy operations",
    title: "Dashboard",
    subtitle: "Operate isolated profiles with fingerprint history, privacy policy and fail-closed network preflight.",
  },
  profiles: {
    eyebrow: "Isolated browser workspaces",
    title: "Profiles",
    subtitle: "Each profile owns browser storage, network routing, fingerprint state and privacy policy.",
  },
  fingerprints: {
    eyebrow: "Per-profile fingerprint engine",
    title: "Fingerprint Center",
    subtitle: "Baseline, history and drift detection remain independent for every profile.",
  },
  privacy: {
    eyebrow: "Policy enforcement & verification",
    title: "Privacy Center",
    subtitle: "Verify applied Chromium policy, network preflight and real-world exposure from the exact selected profile.",
  },
  network: {
    eyebrow: "Connection control",
    title: "Network",
    subtitle: "Review direct/proxy routing and the preflight used by Strict Network Guard.",
  },
  diagnostics: {
    eyebrow: "Runtime health",
    title: "Diagnostics",
    subtitle: "Verify Chromium, WSLg, storage, fingerprint capture and privacy preflight services.",
  },
  settings: {
    eyebrow: "Workspace configuration",
    title: "Settings",
    subtitle: "Review active Dravyn runtime paths and local-only services.",
  },
};

const trackedSurfaceGroups = [
  ["Identity", "User-Agent · Platform · Client Hints · webdriver"],
  ["Rendering", "Canvas hash · WebGL vendor · WebGL renderer"],
  ["Environment", "Screen · DPR · Color depth"],
  ["Regional", "Timezone · Language · Languages"],
  ["Hardware", "CPU concurrency · Device memory · Touch"],
  ["Media", "Audio sample rate · AudioContext"],
  ["Network", "WebRTC candidate types"],
  ["Privacy", "Cookies · Storage · Permissions"],
];

const externalTests: Array<{
  id: ExternalVerificationTest;
  title: string;
  detail: string;
}> = [
  { id: "browserleaks_ip", title: "BrowserLeaks IP", detail: "Public IPv4/IPv6 and network-facing address" },
  { id: "browserleaks_webrtc", title: "BrowserLeaks WebRTC", detail: "Unexpected public/local address exposure" },
  { id: "browserleaks_dns", title: "BrowserLeaks DNS", detail: "Resolver exposure as observed externally" },
  { id: "browserleaks_canvas", title: "BrowserLeaks Canvas", detail: "External canvas observation" },
  { id: "browserleaks_webgl", title: "BrowserLeaks WebGL", detail: "GPU/WebGL surfaces visible to websites" },
  { id: "eff", title: "EFF Cover Your Tracks", detail: "Trackability and fingerprint uniqueness test" },
  { id: "amiunique", title: "AmIUnique", detail: "Attribute-level browser fingerprint comparison" },
];

function presetPolicy(preset: PrivacyPreset): PrivacyPolicy {
  if (preset === "standard") {
    return {
      preset,
      network_guard: "off",
      webrtc: "default",
      block_third_party_cookies: false,
      block_notifications: false,
      block_geolocation: false,
      block_camera: false,
      block_microphone: false,
    };
  }
  if (preset === "strict") {
    return {
      preset,
      network_guard: "strict",
      webrtc: "proxied_only",
      block_third_party_cookies: true,
      block_notifications: true,
      block_geolocation: true,
      block_camera: true,
      block_microphone: true,
    };
  }
  return {
    preset,
    network_guard: "monitor",
    webrtc: "default",
    block_third_party_cookies: true,
    block_notifications: true,
    block_geolocation: false,
    block_camera: false,
    block_microphone: false,
  };
}

const emptyDraft = (): ProfileDraft => ({
  name: "",
  notes: "",
  tags: [],
  browser: {
    start_url: "https://example.com",
    window_width: 1280,
    window_height: 800,
  },
  network: { mode: "direct", proxy: null },
  privacy: presetPolicy("balanced"),
});

function profileToDraft(profile: Profile): ProfileDraft {
  return {
    name: profile.name,
    notes: profile.notes,
    tags: [...profile.tags],
    browser: { ...profile.browser },
    network: {
      mode: profile.network.mode,
      proxy: profile.network.proxy ? { ...profile.network.proxy } : null,
    },
    privacy: { ...profile.privacy },
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function formatRelative(epochSeconds: number | null) {
  if (!epochSeconds) return "Never";
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - epochSeconds);
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

function formatDate(epochSeconds: number) {
  return new Date(epochSeconds * 1000).toLocaleString();
}

function networkLabel(item: ProfileView) {
  const proxy = item.profile.network.proxy;
  if (item.profile.network.mode === "direct") return "Direct connection";
  return proxy ? `${proxy.scheme.toUpperCase()} · ${proxy.host}:${proxy.port}` : "Proxy incomplete";
}

function fingerprintStateLabel(state: FingerprintState) {
  if (state === "stable") return "Stable";
  if (state === "drift") return "Drift detected";
  if (state === "review") return "Review";
  return "Not audited";
}

function fingerprintStateClass(state: FingerprintState) {
  if (state === "stable") return "fpStable";
  if (state === "drift") return "fpDrift";
  if (state === "review") return "fpReview";
  return "fpEmpty";
}

function privacyStatusLabel(status?: PrivacyStatus) {
  if (!status) return "Not checked";
  if (status.overall_status === "critical") return "Critical";
  if (status.overall_status === "restart_required") return "Restart required";
  return "External verify";
}

function privacyStatusClass(status?: PrivacyStatus) {
  if (!status) return "neutral";
  if (status.overall_status === "critical") return "bad";
  if (status.overall_status === "restart_required") return "warn";
  return "good";
}

function App() {
  const [activePage, setActivePage] = useState<Page>("dashboard");
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [profiles, setProfiles] = useState<ProfileView[]>([]);
  const [diagnostics, setDiagnostics] = useState<DiagnosticItem[]>([]);
  const [probes, setProbes] = useState<Record<string, NetworkProbe | undefined>>({});
  const [privacyStatuses, setPrivacyStatuses] = useState<Record<string, PrivacyStatus | undefined>>({});
  const [query, setQuery] = useState("");
  const [sortMode, setSortMode] = useState<SortMode>("updated");
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<ProfileDraft>(emptyDraft());
  const [tagsText, setTagsText] = useState("");
  const [selectedFingerprintId, setSelectedFingerprintId] = useState<string | null>(null);
  const [selectedPrivacyId, setSelectedPrivacyId] = useState<string | null>(null);
  const [fingerprintHistory, setFingerprintHistory] = useState<FingerprintHistoryEntry[]>([]);
  const [fingerprintLatest, setFingerprintLatest] = useState<FingerprintSnapshot | null>(null);
  const [fingerprintLoading, setFingerprintLoading] = useState(false);

  const refresh = useCallback(async () => {
    try {
      setError("");
      const [appStatus, rows, health] = await Promise.all([
        api.appStatus(),
        api.listProfiles(),
        api.systemDiagnostics(),
      ]);
      setStatus(appStatus);
      setProfiles(rows);
      setDiagnostics(health);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const loadFingerprintDetail = useCallback(async (id: string) => {
    try {
      setFingerprintLoading(true);
      const [history, latest] = await Promise.all([
        api.fingerprintHistory(id),
        api.fingerprintLatest(id),
      ]);
      setFingerprintHistory(history);
      setFingerprintLatest(latest);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setFingerprintLoading(false);
    }
  }, []);

  const loadPrivacyStatus = useCallback(async (id: string) => {
    try {
      const result = await api.privacyStatus(id);
      setPrivacyStatuses((current) => ({ ...current, [id]: result }));
      setProbes((current) => ({ ...current, [id]: result.network_probe }));
    } catch (err) {
      setError(errorMessage(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 5000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    if (profiles.length === 0) {
      setSelectedFingerprintId(null);
      setSelectedPrivacyId(null);
      return;
    }
    if (!selectedFingerprintId || !profiles.some((item) => item.profile.id === selectedFingerprintId)) {
      setSelectedFingerprintId(profiles[0].profile.id);
    }
    if (!selectedPrivacyId || !profiles.some((item) => item.profile.id === selectedPrivacyId)) {
      setSelectedPrivacyId(profiles[0].profile.id);
    }
  }, [profiles, selectedFingerprintId, selectedPrivacyId]);

  useEffect(() => {
    if (activePage !== "fingerprints" || !selectedFingerprintId) return;
    void loadFingerprintDetail(selectedFingerprintId);
  }, [activePage, selectedFingerprintId, loadFingerprintDetail]);

  useEffect(() => {
    if (activePage !== "privacy" || !selectedPrivacyId) return;
    void loadPrivacyStatus(selectedPrivacyId);
  }, [activePage, selectedPrivacyId, loadPrivacyStatus]);

  const runningCount = useMemo(
    () => profiles.filter((item) => item.runtime.running).length,
    [profiles],
  );
  const proxyCount = useMemo(
    () => profiles.filter((item) => item.profile.network.mode === "proxy").length,
    [profiles],
  );
  const auditedCount = useMemo(
    () => profiles.filter((item) => item.fingerprint.snapshot_count > 0).length,
    [profiles],
  );
  const driftCount = useMemo(
    () => profiles.filter((item) => item.fingerprint.state === "drift").length,
    [profiles],
  );
  const healthyCount = diagnostics.filter((item) => item.status === "ok").length;

  const visibleProfiles = useMemo(() => {
    const normalized = query.trim().toLowerCase();
    const filtered = normalized
      ? profiles.filter((item) => {
          const haystack = [
            item.profile.name,
            item.profile.notes,
            item.profile.tags.join(" "),
            item.profile.network.proxy?.host ?? "",
            item.profile.privacy.preset,
            fingerprintStateLabel(item.fingerprint.state),
          ]
            .join(" ")
            .toLowerCase();
          return haystack.includes(normalized);
        })
      : [...profiles];

    filtered.sort((left, right) => {
      if (sortMode === "name") return left.profile.name.localeCompare(right.profile.name);
      if (sortMode === "running") return Number(right.runtime.running) - Number(left.runtime.running);
      if (sortMode === "fingerprint") {
        return (right.fingerprint.consistency_score ?? -1) - (left.fingerprint.consistency_score ?? -1);
      }
      return right.profile.updated_at - left.profile.updated_at;
    });
    return filtered;
  }, [profiles, query, sortMode]);

  const selectedFingerprint = useMemo(
    () => profiles.find((item) => item.profile.id === selectedFingerprintId) ?? null,
    [profiles, selectedFingerprintId],
  );
  const selectedPrivacy = useMemo(
    () => profiles.find((item) => item.profile.id === selectedPrivacyId) ?? null,
    [profiles, selectedPrivacyId],
  );

  function openCreate() {
    setEditingId(null);
    setDraft(emptyDraft());
    setTagsText("");
    setEditorOpen(true);
  }

  function openEdit(item: ProfileView) {
    setEditingId(item.profile.id);
    setDraft(profileToDraft(item.profile));
    setTagsText(item.profile.tags.join(", "));
    setEditorOpen(true);
  }

  function openFingerprintCenter(item: ProfileView) {
    setSelectedFingerprintId(item.profile.id);
    setActivePage("fingerprints");
  }

  function openPrivacyCenter(item: ProfileView) {
    setSelectedPrivacyId(item.profile.id);
    setActivePage("privacy");
  }

  async function saveProfile(event: FormEvent) {
    event.preventDefault();
    try {
      setError("");
      const payload: ProfileDraft = {
        ...draft,
        tags: tagsText
          .split(",")
          .map((tag) => tag.trim())
          .filter(Boolean),
      };
      if (editingId) await api.updateProfile(editingId, payload);
      else await api.createProfile(payload);
      setEditorOpen(false);
      await refresh();
      if (editingId) await loadPrivacyStatus(editingId);
    } catch (err) {
      setError(errorMessage(err));
    }
  }

  async function runAction(id: string, action: () => Promise<unknown>) {
    try {
      setBusyId(id);
      setError("");
      await action();
      await refresh();
      if (selectedFingerprintId === id) await loadFingerprintDetail(id);
      if (selectedPrivacyId === id) await loadPrivacyStatus(id);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusyId(null);
    }
  }

  async function deleteProfile(item: ProfileView) {
    if (!window.confirm(`Delete ${item.profile.name}, its browser data and fingerprint history?`)) return;
    await runAction(item.profile.id, () => api.deleteProfile(item.profile.id));
  }

  async function resetProfile(item: ProfileView) {
    if (!window.confirm(`Reset cookies, cache, history and site storage for ${item.profile.name}? Fingerprint history will be preserved.`)) return;
    await runAction(item.profile.id, () => api.resetProfile(item.profile.id));
  }

  async function cloneProfile(item: ProfileView) {
    const copy = profileToDraft(item.profile);
    copy.name = `${item.profile.name} Copy`;
    await runAction(item.profile.id, () => api.createProfile(copy));
  }

  async function probeNetwork(item: ProfileView) {
    try {
      setBusyId(item.profile.id);
      setError("");
      const probe = await api.networkProbe(item.profile.id);
      setProbes((current) => ({ ...current, [item.profile.id]: probe }));
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusyId(null);
    }
  }

  const content = pageCopy[activePage];

  function renderProfileActions(item: ProfileView, compact = false) {
    const busy = busyId === item.profile.id;
    return (
      <div className={`profileActions ${compact ? "compact" : ""}`}>
        {!compact && <button className="button ghost" type="button" disabled={busy} onClick={() => openPrivacyCenter(item)}>Privacy</button>}
        {!compact && <button className="button ghost" type="button" disabled={busy} onClick={() => openFingerprintCenter(item)}>Fingerprint</button>}
        {!compact && <button className="button ghost" type="button" disabled={busy} onClick={() => openEdit(item)}>Edit</button>}
        {!compact && <button className="button ghost" type="button" disabled={busy} onClick={() => void cloneProfile(item)}>Clone</button>}
        {item.runtime.running ? (
          <button className="button stop" type="button" disabled={busy} onClick={() => void runAction(item.profile.id, () => api.stopProfile(item.profile.id))}>{busy ? "Stopping…" : "■ Stop"}</button>
        ) : (
          <button className="button launch" type="button" disabled={busy || !status?.chromium_ready} onClick={() => void runAction(item.profile.id, () => api.launchProfile(item.profile.id))}>{busy ? "Launching…" : "▶ Launch"}</button>
        )}
      </div>
    );
  }

  function renderDashboard() {
    const recent = profiles.slice(0, 4);
    return (
      <div className="pageStack">
        <section className="metricGrid">
          <article className="metricCard accent"><span>Profiles</span><strong>{profiles.length}</strong><small>{runningCount} running now</small></article>
          <article className="metricCard"><span>Fingerprint coverage</span><strong>{auditedCount}/{profiles.length || 0}</strong><small>{driftCount ? `${driftCount} with drift` : "No active drift alerts"}</small></article>
          <article className="metricCard"><span>Privacy policy</span><strong>{profiles.filter((item) => item.profile.privacy.preset === "strict").length}</strong><small>Strict profiles</small></article>
          <article className="metricCard"><span>System health</span><strong>{healthyCount}/{diagnostics.length || 7}</strong><small>Local diagnostics passing</small></article>
        </section>

        <div className="dashboardGrid">
          <section className="surfacePanel">
            <div className="panelHeading"><div><span className="kicker">Recent activity</span><h2>Recent profiles</h2></div><button className="button ghost" type="button" onClick={() => setActivePage("profiles")}>View all</button></div>
            {recent.length ? <div className="recentList">{recent.map((item) => (
              <div className="recentRow" key={item.profile.id}>
                <div className="profileAvatar">{item.profile.name.slice(0, 1).toUpperCase()}</div>
                <div className="recentInfo"><strong>{item.profile.name}</strong><span>{networkLabel(item)} · {item.profile.privacy.preset} privacy</span></div>
                <button className={`fpMini ${fingerprintStateClass(item.fingerprint.state)}`} type="button" onClick={() => openFingerprintCenter(item)}>{item.fingerprint.consistency_score ?? "—"}<small>{fingerprintStateLabel(item.fingerprint.state)}</small></button>
                <span className={`statusChip ${item.runtime.running ? "running" : "stopped"}`}>{item.runtime.running ? "Running" : "Stopped"}</span>
                {renderProfileActions(item, true)}
              </div>
            ))}</div> : <EmptyState title="No profiles yet" description="Create the first isolated browser workspace to start using Dravyn." action={openCreate} />}
          </section>

          <section className="surfacePanel healthPanel">
            <div className="panelHeading"><div><span className="kicker">Local runtime</span><h2>System health</h2></div></div>
            <div className="healthList">{diagnostics.slice(0, 7).map((item) => (
              <div className="healthRow" key={item.id}><span className={`healthDot ${item.status}`} /><div><strong>{item.label}</strong><span>{item.detail}</span></div></div>
            ))}</div>
            <button className="button secondary fullWidth" type="button" onClick={() => setActivePage("diagnostics")}>Open diagnostics</button>
          </section>
        </div>

        <section className="quickPanel">
          <div><span className="kicker">Quick actions</span><h2>Keep work moving</h2></div>
          <div className="quickActions"><button className="quickAction" type="button" onClick={openCreate}><span>＋</span><div><strong>New profile</strong><small>Create isolated storage</small></div></button><button className="quickAction" type="button" onClick={() => setActivePage("privacy")}><span>◈</span><div><strong>Privacy center</strong><small>Policy & external verification</small></div></button><button className="quickAction" type="button" onClick={() => setActivePage("fingerprints")}><span>◇</span><div><strong>Fingerprint center</strong><small>Baseline & drift history</small></div></button></div>
        </section>
      </div>
    );
  }

  function renderProfiles() {
    return (
      <section className="surfacePanel">
        <div className="toolbar">
          <div className="searchBox"><span>⌕</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search profiles, tags, proxy, privacy or fingerprint state…" /></div>
          <select className="selectControl" value={sortMode} onChange={(event) => setSortMode(event.target.value as SortMode)}><option value="updated">Recently updated</option><option value="name">Name A–Z</option><option value="running">Running first</option><option value="fingerprint">Fingerprint score</option></select>
          <button className="button ghost" type="button" onClick={() => void refresh()}>↻ Refresh</button>
        </div>
        {loading ? <div className="loadingState">Loading profiles…</div> : visibleProfiles.length === 0 ? <EmptyState title={profiles.length ? "No matching profiles" : "Create your first profile"} description={profiles.length ? "Try a different search term." : "Each profile receives isolated storage, fingerprint history and its own privacy policy."} action={profiles.length ? undefined : openCreate} /> : <div className="profileGrid">{visibleProfiles.map((item) => {
          const busy = busyId === item.profile.id;
          return <article className="profileCard" key={item.profile.id}>
            <div className="profileTop"><div className="profileAvatar large">{item.profile.name.slice(0, 1).toUpperCase()}</div><div className="profileIdentity"><div className="profileNameLine"><h3>{item.profile.name}</h3><span className={`statusChip ${item.runtime.running ? "running" : "stopped"}`}>{item.runtime.running ? `Running · ${item.runtime.pid ?? ""}` : "Stopped"}</span></div><p>{item.profile.notes || "No notes added"}</p></div></div>
            <div className="profileFacts profileFactsM4"><div><span>Network</span><strong>{networkLabel(item)}</strong></div><div><span>Privacy</span><button className="fpText" type="button" onClick={() => openPrivacyCenter(item)}>{item.profile.privacy.preset} · {item.profile.privacy.network_guard} guard</button></div><div><span>Fingerprint</span><button className={`fpText ${fingerprintStateClass(item.fingerprint.state)}`} type="button" onClick={() => openFingerprintCenter(item)}>{item.fingerprint.consistency_score ?? "—"}/100</button></div><div><span>Updated</span><strong>{formatRelative(item.profile.updated_at)}</strong></div></div>
            {item.profile.tags.length > 0 && <div className="tagRow">{item.profile.tags.map((tag) => <span className="tag" key={tag}>{tag}</span>)}</div>}
            <div className="profileFooter">{renderProfileActions(item)}<div className="moreActions"><button className="textButton" type="button" disabled={busy || item.runtime.running} onClick={() => void resetProfile(item)}>Reset data</button><button className="textButton danger" type="button" disabled={busy || item.runtime.running} onClick={() => void deleteProfile(item)}>Delete</button></div></div>
          </article>;
        })}</div>}
      </section>
    );
  }

  function renderFingerprints() {
    if (profiles.length === 0) return <section className="surfacePanel"><EmptyState title="No profiles available" description="Create a profile first. Fingerprint state always belongs to one concrete profile." action={openCreate} /></section>;
    if (!selectedFingerprint) return <div className="loadingState">Loading fingerprint profile…</div>;

    const summary = selectedFingerprint.fingerprint;
    const busy = busyId === selectedFingerprint.profile.id;
    return (
      <div className="pageStack">
        <section className="fingerprintProfileBar">
          <div className="fingerprintProfileSelect"><span className="profileAvatar large">{selectedFingerprint.profile.name.slice(0, 1).toUpperCase()}</span><div><span className="kicker">Active fingerprint workspace</span><select value={selectedFingerprint.profile.id} onChange={(event) => setSelectedFingerprintId(event.target.value)}>{profiles.map((item) => <option key={item.profile.id} value={item.profile.id}>{item.profile.name}</option>)}</select></div></div>
          <div className="fingerprintProfileActions"><button className="button ghost" type="button" disabled={busy || !summary.snapshot_count} onClick={() => { if (window.confirm("Replace this profile's baseline with the latest captured snapshot?")) void runAction(selectedFingerprint.profile.id, () => api.setFingerprintBaseline(selectedFingerprint.profile.id)); }}>Set latest as baseline</button><button className="button launch" type="button" disabled={busy || !status?.chromium_ready} onClick={() => void runAction(selectedFingerprint.profile.id, () => api.openPrivacyAudit(selectedFingerprint.profile.id))}>◇ Run profile audit</button></div>
        </section>

        <section className={`fingerprintHero ${fingerprintStateClass(summary.state)}`}>
          <div className="fingerprintScore"><span>Consistency</span><strong>{summary.consistency_score ?? "—"}</strong><small>{summary.consistency_score === null ? "Run the first audit" : "/100"}</small></div>
          <div className="fingerprintHeroCopy"><span className="kicker">{fingerprintStateLabel(summary.state)}</span><h2>{selectedFingerprint.profile.name}</h2><p>{summary.state === "not_audited" ? "No baseline exists yet. The first successful local audit creates one for this profile." : summary.state === "drift" ? `${summary.drift_count} stable browser surfaces changed since the current baseline.` : summary.state === "review" ? `${summary.issue_count} consistency/privacy items need review.` : "Stable tracked surfaces match this profile's current baseline."}</p><div className="fingerprintMeta"><span>Baseline {summary.baseline_present ? "locked" : "pending"}</span><span>{summary.snapshot_count} snapshots</span><span>{summary.surface_count} surfaces</span><span>Last audit {formatRelative(summary.last_captured_at)}</span></div></div>
          <div className="fingerprintShield"><span>◇</span><strong>Profile-local</strong><small>Baseline + history</small></div>
        </section>

        <section className="fingerprintMetricGrid">
          <article><span>Baseline</span><strong>{summary.baseline_present ? "Ready" : "Not created"}</strong><small>First audit creates it automatically</small></article>
          <article><span>Stable drift</span><strong>{summary.drift_count}</strong><small>Tracked surface changes</small></article>
          <article><span>Review items</span><strong>{summary.issue_count}</strong><small>Current consistency warnings</small></article>
          <article><span>History</span><strong>{summary.snapshot_count}</strong><small>Up to 50 local snapshots</small></article>
        </section>

        <div className="fingerprintWorkspaceGrid">
          <section className="surfacePanel"><div className="panelHeading"><div><span className="kicker">Profile timeline</span><h2>Fingerprint history</h2></div><button className="button ghost" type="button" disabled={fingerprintLoading} onClick={() => void loadFingerprintDetail(selectedFingerprint.profile.id)}>↻ Refresh</button></div>{fingerprintLoading && fingerprintHistory.length === 0 ? <div className="loadingState">Loading fingerprint history…</div> : fingerprintHistory.length === 0 ? <div className="fingerprintEmpty"><strong>No snapshots yet</strong><span>Run the profile audit to establish the first baseline.</span></div> : <div className="fingerprintHistoryList">{fingerprintHistory.map((entry, index) => <div className="fingerprintHistoryRow" key={entry.snapshot_id}><div className={`historyScore ${entry.drift_count ? "bad" : entry.issue_count ? "warn" : "good"}`}>{entry.consistency_score}</div><div><strong>{index === 0 ? "Latest snapshot" : `Snapshot ${fingerprintHistory.length - index}`}</strong><span>{formatDate(entry.captured_at)}</span></div><div className="historyFacts"><span>{entry.surface_count} surfaces</span><span>{entry.drift_count} drift</span><span>{entry.issue_count} review</span></div></div>)}</div>}</section>
          <section className="surfacePanel"><div className="panelHeading"><div><span className="kicker">Latest comparison</span><h2>Drift & consistency</h2></div></div>{!fingerprintLatest ? <div className="fingerprintEmpty"><strong>Waiting for first audit</strong><span>Later snapshots will be compared with the first stable baseline.</span></div> : <div className="fingerprintDetailBody">{fingerprintLatest.drift.length > 0 ? <div className="driftList">{fingerprintLatest.drift.map((item) => <article className="driftItem" key={item.key}><div><span>{item.category}</span><strong>{item.label}</strong></div><code>{item.baseline_value}</code><b>→</b><code>{item.current_value}</code></article>)}</div> : <div className="stableCallout"><span>✓</span><div><strong>No stable-surface drift</strong><p>The latest tracked values match this profile's baseline.</p></div></div>}{fingerprintLatest.issues.length > 0 && <div className="issueStack"><h3>Review items</h3>{fingerprintLatest.issues.map((issue) => <p key={issue}>! {issue}</p>)}</div>}</div>}</section>
        </div>

        <section className="surfacePanel"><div className="panelHeading"><div><span className="kicker">Audit coverage</span><h2>Tracked surface groups</h2></div></div><div className="surfaceGrid">{trackedSurfaceGroups.map(([title, detail]) => <div className="surfaceTile" key={title}><span className="surfaceDot" /><div><strong>{title}</strong><p>{detail}</p></div></div>)}</div></section>
      </div>
    );
  }

  function renderPrivacy() {
    if (profiles.length === 0) return <section className="surfacePanel"><EmptyState title="No profiles available" description="Create a profile before configuring privacy policy and verification." action={openCreate} /></section>;
    if (!selectedPrivacy) return <div className="loadingState">Loading privacy profile…</div>;
    const current = privacyStatuses[selectedPrivacy.profile.id];
    const busy = busyId === selectedPrivacy.profile.id;
    return (
      <div className="pageStack">
        <section className="privacyProfileBar">
          <div className="fingerprintProfileSelect"><span className="profileAvatar large">{selectedPrivacy.profile.name.slice(0, 1).toUpperCase()}</span><div><span className="kicker">Active privacy workspace</span><select value={selectedPrivacy.profile.id} onChange={(event) => setSelectedPrivacyId(event.target.value)}>{profiles.map((item) => <option key={item.profile.id} value={item.profile.id}>{item.profile.name}</option>)}</select></div></div>
          <div className="fingerprintProfileActions"><button className="button ghost" type="button" onClick={() => openEdit(selectedPrivacy)}>Edit policy</button><button className="button secondary" type="button" disabled={busy} onClick={() => void loadPrivacyStatus(selectedPrivacy.profile.id)}>↻ Run preflight</button></div>
        </section>

        <section className={`privacyHeroM5 ${privacyStatusClass(current)}`}>
          <div className="privacyHeroIcon">◈</div>
          <div><span className="kicker">{privacyStatusLabel(current)}</span><h2>{selectedPrivacy.profile.name}</h2><p>{current?.message ?? "Run preflight to verify stored Chromium preferences and the selected network route."}</p><div className="fingerprintMeta"><span>{selectedPrivacy.profile.privacy.preset} preset</span><span>{selectedPrivacy.profile.privacy.network_guard} network guard</span><span>WebRTC {selectedPrivacy.profile.privacy.webrtc}</span></div></div>
          <button className="button launch" type="button" disabled={busy || !status?.chromium_ready} onClick={() => void runAction(selectedPrivacy.profile.id, () => api.launchProfile(selectedPrivacy.profile.id))}>{selectedPrivacy.runtime.running ? "Profile running" : "▶ Launch with policy"}</button>
        </section>

        <section className="privacyMetricGrid">
          <article><span>Policy storage</span><strong>{current?.policy_applied.applied ? "Applied" : "Needs launch"}</strong><small>{current?.policy_applied.message ?? "Not checked"}</small></article>
          <article><span>Network guard</span><strong>{selectedPrivacy.profile.privacy.network_guard}</strong><small>{current?.network_probe.message ?? "Run preflight"}</small></article>
          <article><span>WebRTC policy</span><strong>{selectedPrivacy.profile.privacy.webrtc}</strong><small>{current?.policy_applied.actual_webrtc_policy ?? "Not applied yet"}</small></article>
          <article><span>External verification</span><strong>Required</strong><small>Local checks cannot prove what a remote website sees</small></article>
        </section>

        <div className="privacyWorkspaceGrid">
          <section className="surfacePanel">
            <div className="panelHeading"><div><span className="kicker">Enforced before launch</span><h2>Per-profile policy</h2></div></div>
            <div className="policyRows">
              <PolicyRow label="Third-party cookies" value={selectedPrivacy.profile.privacy.block_third_party_cookies ? "Block" : "Allow/default"} />
              <PolicyRow label="Notifications" value={selectedPrivacy.profile.privacy.block_notifications ? "Block" : "Default"} />
              <PolicyRow label="Geolocation" value={selectedPrivacy.profile.privacy.block_geolocation ? "Block" : "Default"} />
              <PolicyRow label="Camera" value={selectedPrivacy.profile.privacy.block_camera ? "Block" : "Default"} />
              <PolicyRow label="Microphone" value={selectedPrivacy.profile.privacy.block_microphone ? "Block" : "Default"} />
              <PolicyRow label="Preferences file" value={current?.policy_applied.preferences_path ?? "Run preflight"} />
            </div>
          </section>

          <section className="surfacePanel">
            <div className="panelHeading"><div><span className="kicker">Fail-closed route check</span><h2>Network preflight</h2></div></div>
            <div className="privacyPreflight"><span className={`preflightLight ${current?.network_probe.reachable === false ? "bad" : current?.network_probe.reachable === true ? "good" : "neutral"}`} /><div><strong>{networkLabel(selectedPrivacy)}</strong><p>{current?.network_probe.message ?? "Run preflight to validate the configured route."}</p></div></div>
            <div className="privacyNote">Strict guard blocks launch only when a configured proxy fails endpoint preflight. Public IP, DNS, IPv6 and WebRTC exposure still need a real website test below.</div>
          </section>
        </div>

        <section className="surfacePanel">
          <div className="panelHeading"><div><span className="kicker">Real website view</span><h2>External Verification Lab</h2></div><span className="softCount">Opens inside {selectedPrivacy.profile.name}</span></div>
          <div className="verificationGrid">{externalTests.map((test) => <button className="verificationCard" type="button" disabled={busy || !status?.chromium_ready} key={test.id} onClick={() => void runAction(selectedPrivacy.profile.id, () => api.openExternalVerification(selectedPrivacy.profile.id, test.id))}><span>↗</span><div><strong>{test.title}</strong><small>{test.detail}</small></div></button>)}</div>
        </section>

        <section className="scopeBanner"><strong>Verification rule</strong><span>A local policy check is not proof of anonymity or zero leakage. Confirm IP/WebRTC/DNS/IPv6 with an external test running inside the exact profile and treat unexpected real-network exposure as critical.</span></section>
      </div>
    );
  }

  function renderNetwork() {
    return (
      <div className="pageStack">
        <section className="infoBanner"><div className="infoIcon">⌁</div><div><strong>Preflight is not anonymity verification</strong><p>Dravyn resolves the configured proxy host and attempts a short TCP connection. Strict Network Guard can fail closed on an unreachable proxy, while remote leak verification remains a separate Privacy Center step.</p></div></section>
        <section className="surfacePanel">
          <div className="panelHeading"><div><span className="kicker">Profile routing</span><h2>Connections</h2></div><span className="softCount">{proxyCount} proxy · {profiles.length - proxyCount} direct</span></div>
          {profiles.length === 0 ? <EmptyState title="No profiles to inspect" description="Create a profile first, then configure Direct or Explicit proxy routing." action={openCreate} /> : <div className="networkList">{profiles.map((item) => {
            const probe = probes[item.profile.id];
            const busy = busyId === item.profile.id;
            return <article className="networkRow" key={item.profile.id}><div className="networkIdentity"><div className="profileAvatar">{item.profile.name.slice(0, 1).toUpperCase()}</div><div><strong>{item.profile.name}</strong><span>{networkLabel(item)} · guard {item.profile.privacy.network_guard}</span></div></div><div className="networkState"><span className={`routeBadge ${item.profile.network.mode}`}>{item.profile.network.mode === "direct" ? "DIRECT" : "PROXY"}</span>{probe && <span className={`probeBadge ${probe.reachable === false ? "bad" : probe.reachable === true ? "good" : "neutral"}`}>{probe.reachable === true ? `Reachable · ${probe.latency_ms ?? 0}ms` : probe.reachable === false ? "Unreachable" : "Configuration valid"}</span>}</div><div className="networkActions"><button className="button ghost" type="button" onClick={() => openPrivacyCenter(item)}>Privacy</button><button className="button secondary" disabled={busy} type="button" onClick={() => void probeNetwork(item)}>{busy ? "Testing…" : "Test preflight"}</button></div>{probe && <p className="probeMessage">{probe.message}</p>}</article>;
          })}</div>}
        </section>
      </div>
    );
  }

  function renderDiagnostics() {
    return <section className="surfacePanel"><div className="panelHeading"><div><span className="kicker">Local environment</span><h2>System checks</h2></div><button className="button ghost" type="button" onClick={() => void refresh()}>↻ Run again</button></div><div className="diagnosticGrid">{diagnostics.map((item) => <article className="diagnosticCard" key={item.id}><div className={`diagnosticIcon ${item.status}`}>{item.status === "ok" ? "✓" : item.status === "warning" ? "!" : "×"}</div><div><span className="diagStatus">{item.status}</span><h3>{item.label}</h3><p>{item.detail}</p></div></article>)}</div></section>;
  }

  function renderSettings() {
    return <div className="settingsGrid"><section className="surfacePanel settingsPanel"><div className="panelHeading"><div><span className="kicker">Runtime</span><h2>Workspace</h2></div></div><SettingsRow label="Dravyn home" value={status?.workspace ?? "Checking…"} /><SettingsRow label="Chromium binary" value={status?.browser_binary ?? "Checking…"} /><SettingsRow label="Chromium state" value={status?.chromium_state ?? "Checking…"} /><SettingsRow label="Fingerprint capture" value={status?.fingerprint_capture_origin ?? "Checking…"} /><SettingsRow label="Desktop version" value={status?.version ?? "0.1.0"} /></section><section className="surfacePanel settingsPanel"><div className="panelHeading"><div><span className="kicker">M5 enforcement</span><h2>Privacy model</h2></div></div><p className="settingsCopy">Privacy policy is stored per profile. Before a stopped profile launches, Dravyn writes the selected Chromium preferences, verifies them, and runs fail-closed proxy preflight when Strict Network Guard is enabled.</p><div className="settingsNote"><strong>External truth remains separate</strong><span>Only a remote website can confirm the public IP, DNS, IPv6 and WebRTC values visible outside the device, so the Verification Lab opens those tests inside the selected profile.</span></div></section></div>;
  }

  return (
    <div className="appShell">
      <aside className="sidebar">
        <button className="brand" type="button" onClick={() => setActivePage("dashboard")}><span className="brandMark">D</span><span className="brandText"><strong>Dravyn</strong><small>Browser Core</small></span></button>
        <nav className="navList">{navItems.map((item) => <button className={`navItem ${activePage === item.id ? "active" : ""}`} type="button" key={item.id} onClick={() => setActivePage(item.id)}><span className="navIcon">{item.icon}</span><span>{item.label}</span>{item.id === "profiles" && profiles.length > 0 && <em>{profiles.length}</em>}{item.id === "fingerprints" && driftCount > 0 && <em className="navAlert">{driftCount}</em>}</button>)}</nav>
        <div className="sidebarBottom"><div className={`sidebarHealth ${status?.chromium_ready ? "ready" : "warning"}`}><span className="healthPulse" /><div><strong>Chromium {status?.chromium_ready ? "ready" : "not ready"}</strong><small>{status?.chromium_state ?? "Checking…"}</small></div></div><div className="versionLine">M5 · v{status?.version ?? "0.1.0"}</div></div>
      </aside>

      <main className="mainContent">
        <header className="topbar"><div className="titleBlock"><span className="eyebrow">{content.eyebrow}</span><h1>{content.title}</h1><p>{content.subtitle}</p></div><div className="topActions"><span className={`browserPill ${status?.chromium_ready ? "ready" : "notReady"}`}><span className="dot" /> Chromium {status?.chromium_ready ? "Ready" : "Not ready"}</span><button className="button primary" type="button" onClick={openCreate}>＋ New profile</button></div></header>
        {error && <div className="errorBanner" role="alert"><span>!</span><div><strong>Dravyn needs attention</strong><p>{error}</p></div><button type="button" onClick={() => setError("")}>×</button></div>}
        <div className="pageBody">{activePage === "dashboard" && renderDashboard()}{activePage === "profiles" && renderProfiles()}{activePage === "fingerprints" && renderFingerprints()}{activePage === "privacy" && renderPrivacy()}{activePage === "network" && renderNetwork()}{activePage === "diagnostics" && renderDiagnostics()}{activePage === "settings" && renderSettings()}</div>
      </main>

      {editorOpen && <div className="modalBackdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setEditorOpen(false); }}><form className="modal" onSubmit={(event) => void saveProfile(event)}><div className="modalHeader"><div><span className="eyebrow">Profile configuration</span><h2>{editingId ? "Edit profile" : "Create profile"}</h2><p>Storage, network, fingerprint history and privacy policy remain attached to this workspace.</p></div><button className="iconButton" type="button" onClick={() => setEditorOpen(false)} aria-label="Close">×</button></div><div className="formGrid"><label className="full"><span>Name</span><input required maxLength={80} value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} placeholder="Work profile" /></label><label className="full"><span>Notes</span><textarea maxLength={4000} rows={3} value={draft.notes} onChange={(event) => setDraft({ ...draft, notes: event.target.value })} placeholder="Purpose or workflow notes" /></label><label className="full"><span>Tags <small>comma separated</small></span><input value={tagsText} onChange={(event) => setTagsText(event.target.value)} placeholder="work, qa, client-a" /></label><label className="full"><span>Start URL</span><input value={draft.browser.start_url ?? ""} onChange={(event) => setDraft({ ...draft, browser: { ...draft.browser, start_url: event.target.value || null } })} placeholder="https://example.com" /></label><label><span>Window width</span><input type="number" min={640} max={7680} value={draft.browser.window_width ?? ""} onChange={(event) => setDraft({ ...draft, browser: { ...draft.browser, window_width: event.target.value ? Number(event.target.value) : null } })} /></label><label><span>Window height</span><input type="number" min={480} max={4320} value={draft.browser.window_height ?? ""} onChange={(event) => setDraft({ ...draft, browser: { ...draft.browser, window_height: event.target.value ? Number(event.target.value) : null } })} /></label>
        <div className="formDivider full"><span>Network routing</span></div><label className="full"><span>Connection mode</span><select value={draft.network.mode} onChange={(event) => { const mode = event.target.value as NetworkMode; setDraft({ ...draft, network: mode === "direct" ? { mode, proxy: null } : { mode, proxy: draft.network.proxy ?? { scheme: "http", host: "127.0.0.1", port: 8080 } } }); }}><option value="direct">Direct connection</option><option value="proxy">Explicit proxy</option></select></label>{draft.network.mode === "proxy" && draft.network.proxy && <><label><span>Proxy scheme</span><select value={draft.network.proxy.scheme} onChange={(event) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, scheme: event.target.value as ProxyScheme } } })}><option value="http">HTTP</option><option value="https">HTTPS</option><option value="socks5">SOCKS5</option></select></label><label><span>Proxy port</span><input type="number" min={1} max={65535} required value={draft.network.proxy.port} onChange={(event) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, port: Number(event.target.value) } } })} /></label><label className="full"><span>Proxy host</span><input required value={draft.network.proxy.host} onChange={(event) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, host: event.target.value } } })} placeholder="127.0.0.1" /></label></>}
        <div className="formDivider full"><span>Privacy policy</span></div><label><span>Preset</span><select value={draft.privacy.preset} onChange={(event) => { const preset = event.target.value as PrivacyPreset; setDraft({ ...draft, privacy: preset === "custom" ? { ...draft.privacy, preset } : presetPolicy(preset) }); }}><option value="standard">Standard</option><option value="balanced">Balanced</option><option value="strict">Strict</option><option value="custom">Custom</option></select></label><label><span>Network guard</span><select value={draft.privacy.network_guard} onChange={(event) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", network_guard: event.target.value as NetworkGuardMode } })}><option value="off">Off</option><option value="monitor">Monitor</option><option value="strict">Strict / fail closed</option></select></label><label className="full"><span>WebRTC network policy</span><select value={draft.privacy.webrtc} onChange={(event) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", webrtc: event.target.value as WebRtcPolicy } })}><option value="default">Chromium default</option><option value="proxied_only">Disable non-proxied UDP</option></select></label><div className="privacyChecks full"><Check label="Block third-party cookies" checked={draft.privacy.block_third_party_cookies} onChange={(checked) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", block_third_party_cookies: checked } })} /><Check label="Block notifications" checked={draft.privacy.block_notifications} onChange={(checked) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", block_notifications: checked } })} /><Check label="Block geolocation" checked={draft.privacy.block_geolocation} onChange={(checked) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", block_geolocation: checked } })} /><Check label="Block camera" checked={draft.privacy.block_camera} onChange={(checked) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", block_camera: checked } })} /><Check label="Block microphone" checked={draft.privacy.block_microphone} onChange={(checked) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", block_microphone: checked } })} /></div><p className="fieldHint full">Privacy changes are verified before the next stopped-profile launch. Stop and relaunch a running profile after changing its policy.</p></div><div className="modalActions"><button className="button ghost" type="button" onClick={() => setEditorOpen(false)}>Cancel</button><button className="button primary" type="submit">{editingId ? "Save changes" : "Create profile"}</button></div></form></div>}
    </div>
  );
}

function EmptyState({ title, description, action }: { title: string; description: string; action?: () => void }) {
  return <div className="emptyState"><div className="emptySymbol">◎</div><h3>{title}</h3><p>{description}</p>{action && <button className="button primary" type="button" onClick={action}>＋ New profile</button>}</div>;
}

function SettingsRow({ label, value }: { label: string; value: string }) {
  return <div className="settingsRow"><span>{label}</span><code>{value}</code></div>;
}

function PolicyRow({ label, value }: { label: string; value: string }) {
  return <div className="policyRow"><span>{label}</span><strong>{value}</strong></div>;
}

function Check({ label, checked, onChange }: { label: string; checked: boolean; onChange: (checked: boolean) => void }) {
  return <label className="privacyCheck"><input type="checkbox" checked={checked} onChange={(event) => onChange(event.target.checked)} /><span>{label}</span></label>;
}

export default App;
