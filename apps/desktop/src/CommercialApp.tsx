import {
  FormEvent,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";
import { api } from "./api";
import type {
  AppStatus,
  DiagnosticItem,
  ExternalVerificationTest,
  FingerprintHistoryEntry,
  FingerprintSnapshot,
  NetworkMode,
  NetworkProbe,
  PrivacyPreset,
  PrivacyStatus,
  Profile,
  ProfileDraft,
  ProfileView,
  ProxyScheme,
  VerificationDraft,
  VerificationRecord,
  VerificationResult,
} from "./types";

const PAGE_IDS = [
  "overview",
  "profiles",
  "privacy",
  "fingerprints",
  "verification",
  "network",
  "diagnostics",
  "settings",
] as const;

type Page = (typeof PAGE_IDS)[number];

type TestDefinition = {
  id: ExternalVerificationTest;
  title: string;
  subtitle: string;
  group: "network" | "fingerprint";
  core: boolean;
  url: string;
};

const TESTS: TestDefinition[] = [
  {
    id: "browserleaks_ip",
    title: "Public IP",
    subtitle: "Confirm IPv4 visible to a remote website",
    group: "network",
    core: true,
    url: "https://browserleaks.com/ip",
  },
  {
    id: "browserleaks_webrtc",
    title: "WebRTC exposure",
    subtitle: "Review candidate addresses and unexpected public routes",
    group: "network",
    core: true,
    url: "https://browserleaks.com/webrtc",
  },
  {
    id: "browserleaks_dns",
    title: "DNS exposure",
    subtitle: "Review resolvers visible from the remote test",
    group: "network",
    core: true,
    url: "https://browserleaks.com/dns",
  },
  {
    id: "browserleaks_ipv6",
    title: "IPv6 exposure",
    subtitle: "Confirm no unexpected native IPv6 path is visible",
    group: "network",
    core: true,
    url: "https://browserleaks.com/ip",
  },
  {
    id: "browserleaks_canvas",
    title: "Canvas",
    subtitle: "Compare remote canvas observations with local stability",
    group: "fingerprint",
    core: false,
    url: "https://browserleaks.com/canvas",
  },
  {
    id: "browserleaks_webgl",
    title: "WebGL",
    subtitle: "Review graphics capabilities exposed to websites",
    group: "fingerprint",
    core: false,
    url: "https://browserleaks.com/webgl",
  },
  {
    id: "eff",
    title: "EFF Cover Your Tracks",
    subtitle: "Independent browser trackability test",
    group: "fingerprint",
    core: false,
    url: "https://coveryourtracks.eff.org/",
  },
  {
    id: "amiunique",
    title: "AmIUnique",
    subtitle: "Independent fingerprint uniqueness report",
    group: "fingerprint",
    core: false,
    url: "https://amiunique.org/fingerprint",
  },
];

const NAV: Array<{
  id: Page;
  icon: string;
  label: string;
  section: "operate" | "assure" | "system";
}> = [
  { id: "overview", icon: "⌂", label: "Overview", section: "operate" },
  { id: "profiles", icon: "◉", label: "Profiles", section: "operate" },
  { id: "privacy", icon: "◈", label: "Privacy", section: "assure" },
  { id: "fingerprints", icon: "◇", label: "Fingerprints", section: "assure" },
  { id: "verification", icon: "✓", label: "Verification", section: "assure" },
  { id: "network", icon: "⌁", label: "Network", section: "assure" },
  { id: "diagnostics", icon: "▤", label: "Diagnostics", section: "system" },
  { id: "settings", icon: "⚙", label: "Settings", section: "system" },
];

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
  privacy: {
    preset: "balanced",
    network_guard: "monitor",
    webrtc: "default",
    block_third_party_cookies: true,
    block_notifications: true,
    block_geolocation: false,
    block_camera: false,
    block_microphone: false,
  },
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

function presetPolicy(preset: PrivacyPreset, current: ProfileDraft["privacy"]) {
  if (preset === "standard") {
    return {
      ...current,
      preset,
      network_guard: "off" as const,
      webrtc: "default" as const,
      block_third_party_cookies: false,
      block_notifications: false,
      block_geolocation: false,
      block_camera: false,
      block_microphone: false,
    };
  }
  if (preset === "strict") {
    return {
      ...current,
      preset,
      network_guard: "strict" as const,
      webrtc: "proxied_only" as const,
      block_third_party_cookies: true,
      block_notifications: true,
      block_geolocation: true,
      block_camera: true,
      block_microphone: true,
    };
  }
  if (preset === "balanced") {
    return {
      ...current,
      preset,
      network_guard: "monitor" as const,
      webrtc: "default" as const,
      block_third_party_cookies: true,
      block_notifications: true,
      block_geolocation: false,
      block_camera: false,
      block_microphone: false,
    };
  }
  return { ...current, preset };
}

function formatRelative(epoch: number | null | undefined) {
  if (!epoch) return "Never";
  const seconds = Math.max(0, Math.floor(Date.now() / 1000) - epoch);
  if (seconds < 60) return `${seconds}s ago`;
  if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
  if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
  return `${Math.floor(seconds / 86400)}d ago`;
}

function formatDate(epoch: number) {
  return new Date(epoch * 1000).toLocaleString();
}

function errorText(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

function routeLabel(item: ProfileView) {
  const proxy = item.profile.network.proxy;
  if (item.profile.network.mode === "direct") return "Direct";
  if (!proxy) return "Proxy incomplete";
  return `${proxy.scheme.toUpperCase()} · ${proxy.host}:${proxy.port}`;
}

function verificationLabel(state: ProfileView["verification"]["state"]) {
  if (state === "healthy") return "Verified";
  if (state === "critical") return "Critical";
  if (state === "review") return "Review";
  return "Unverified";
}

function fingerprintLabel(item: ProfileView) {
  const fp = item.fingerprint;
  if (fp.state === "stable") return `Stable · ${fp.consistency_score ?? "—"}`;
  if (fp.state === "drift") return `Drift · ${fp.drift_count}`;
  if (fp.state === "review") return `Review · ${fp.issue_count}`;
  return "Not audited";
}

function profileHealth(item: ProfileView) {
  if (item.verification.state === "critical") {
    return { state: "critical", label: "Critical", detail: "Verification leak/critical result" };
  }
  if (item.fingerprint.state === "drift") {
    return { state: "review", label: "Review", detail: "Fingerprint drift detected" };
  }
  if (item.verification.state === "healthy" && item.fingerprint.state === "stable") {
    return { state: "healthy", label: "Healthy", detail: "Stable and externally reviewed" };
  }
  return { state: "pending", label: "Needs review", detail: "Complete privacy verification" };
}

function resultLabel(result: VerificationResult) {
  if (result === "pass") return "Pass";
  if (result === "warning") return "Warning";
  if (result === "critical") return "Critical";
  return "Inconclusive";
}

export default function CommercialApp() {
  const [page, setPage] = useState<Page>("overview");
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [profiles, setProfiles] = useState<ProfileView[]>([]);
  const [diagnostics, setDiagnostics] = useState<DiagnosticItem[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [privacyStatus, setPrivacyStatus] = useState<PrivacyStatus | null>(null);
  const [fingerprintHistory, setFingerprintHistory] = useState<FingerprintHistoryEntry[]>([]);
  const [fingerprintLatest, setFingerprintLatest] = useState<FingerprintSnapshot | null>(null);
  const [verificationHistory, setVerificationHistory] = useState<VerificationRecord[]>([]);
  const [probes, setProbes] = useState<Record<string, NetworkProbe>>({});
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [toast, setToast] = useState("");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<ProfileDraft>(emptyDraft());
  const [tagsText, setTagsText] = useState("");
  const [verificationModal, setVerificationModal] = useState<TestDefinition | null>(null);
  const [verificationDraft, setVerificationDraft] = useState<VerificationDraft>({
    test: "browserleaks_ip",
    result: "pass",
    expected: null,
    observed: null,
    notes: "",
    source_url: null,
    chromium_version: null,
    policy_version: 1,
  });
  const [commandOpen, setCommandOpen] = useState(false);
  const [commandQuery, setCommandQuery] = useState("");
  const [compareLeft, setCompareLeft] = useState<string | null>(null);
  const [compareRight, setCompareRight] = useState<string | null>(null);
  const [compareSnapshots, setCompareSnapshots] = useState<{
    left: FingerprintSnapshot | null;
    right: FingerprintSnapshot | null;
  }>({ left: null, right: null });

  const refresh = useCallback(async () => {
    try {
      setError("");
      const [app, rows, health] = await Promise.all([
        api.appStatus(),
        api.listProfiles(),
        api.systemDiagnostics(),
      ]);
      setStatus(app);
      setProfiles(rows);
      setDiagnostics(health);
      setSelectedId((current) => {
        if (current && rows.some((item) => item.profile.id === current)) return current;
        return rows[0]?.profile.id ?? null;
      });
      setCompareLeft((current) => current ?? rows[0]?.profile.id ?? null);
      setCompareRight((current) => current ?? rows[1]?.profile.id ?? rows[0]?.profile.id ?? null);
    } catch (err) {
      setError(errorText(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const selected = useMemo(
    () => profiles.find((item) => item.profile.id === selectedId) ?? null,
    [profiles, selectedId],
  );

  const filteredProfiles = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return profiles;
    return profiles.filter((item) =>
      [
        item.profile.name,
        item.profile.notes,
        item.profile.tags.join(" "),
        routeLabel(item),
        verificationLabel(item.verification.state),
      ]
        .join(" ")
        .toLowerCase()
        .includes(needle),
    );
  }, [profiles, query]);

  const runningCount = profiles.filter((item) => item.runtime.running).length;
  const criticalCount = profiles.filter(
    (item) => profileHealth(item).state === "critical",
  ).length;
  const verifiedCount = profiles.filter(
    (item) => item.verification.state === "healthy",
  ).length;
  const stableCount = profiles.filter((item) => item.fingerprint.state === "stable").length;

  const loadSelectedDetail = useCallback(async (id: string) => {
    try {
      const [privacy, history, latest, verifications] = await Promise.all([
        api.privacyStatus(id),
        api.fingerprintHistory(id),
        api.fingerprintLatest(id),
        api.verificationHistory(id),
      ]);
      setPrivacyStatus(privacy);
      setFingerprintHistory(history);
      setFingerprintLatest(latest);
      setVerificationHistory(verifications);
    } catch (err) {
      setError(errorText(err));
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 6000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  useEffect(() => {
    if (!selectedId) {
      setPrivacyStatus(null);
      setFingerprintHistory([]);
      setFingerprintLatest(null);
      setVerificationHistory([]);
      return;
    }
    if (["privacy", "fingerprints", "verification"].includes(page)) {
      void loadSelectedDetail(selectedId);
    }
  }, [selectedId, page, loadSelectedDetail]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "k") {
        event.preventDefault();
        setCommandOpen((value) => !value);
      }
      if (event.key === "Escape") {
        setCommandOpen(false);
        setEditorOpen(false);
        setVerificationModal(null);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(""), 2600);
    return () => window.clearTimeout(timer);
  }, [toast]);

  async function act(id: string, fn: () => Promise<unknown>, success?: string) {
    try {
      setBusy(id);
      setError("");
      await fn();
      if (success) setToast(success);
      await refresh();
      if (selectedId === id && ["privacy", "fingerprints", "verification"].includes(page)) {
        await loadSelectedDetail(id);
      }
    } catch (err) {
      setError(errorText(err));
    } finally {
      setBusy(null);
    }
  }

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

  async function saveProfile(event: FormEvent) {
    event.preventDefault();
    const payload: ProfileDraft = {
      ...draft,
      tags: tagsText
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
    };
    try {
      setBusy(editingId ?? "new");
      if (editingId) await api.updateProfile(editingId, payload);
      else await api.createProfile(payload);
      setEditorOpen(false);
      setToast(editingId ? "Profile updated" : "Profile created");
      await refresh();
    } catch (err) {
      setError(errorText(err));
    } finally {
      setBusy(null);
    }
  }

  async function deleteProfile(item: ProfileView) {
    if (!window.confirm(`Delete ${item.profile.name} and all local browser/fingerprint/verification data?`)) return;
    await act(item.profile.id, () => api.deleteProfile(item.profile.id), "Profile deleted");
  }

  async function resetProfile(item: ProfileView) {
    if (!window.confirm(`Reset cookies, cache and site data for ${item.profile.name}? Fingerprint and verification history are preserved.`)) return;
    await act(item.profile.id, () => api.resetProfile(item.profile.id), "Browser data reset");
  }

  async function probe(item: ProfileView) {
    try {
      setBusy(item.profile.id);
      const result = await api.networkProbe(item.profile.id);
      setProbes((current) => ({ ...current, [item.profile.id]: result }));
      setToast(result.reachable === false ? "Network preflight failed" : "Network preflight complete");
    } catch (err) {
      setError(errorText(err));
    } finally {
      setBusy(null);
    }
  }

  function focusProfile(item: ProfileView, target: Page) {
    setSelectedId(item.profile.id);
    setPage(target);
  }

  async function openTest(test: TestDefinition) {
    if (!selected) return;
    await act(
      selected.profile.id,
      () => api.openExternalVerification(selected.profile.id, test.id),
      `${test.title} opened in ${selected.profile.name}`,
    );
  }

  function openResult(test: TestDefinition) {
    setVerificationModal(test);
    setVerificationDraft({
      test: test.id,
      result: "pass",
      expected: null,
      observed: null,
      notes: "",
      source_url: test.url,
      chromium_version: status?.chromium_state ?? null,
      policy_version: 1,
    });
  }

  async function saveVerification(event: FormEvent) {
    event.preventDefault();
    if (!selected || !verificationModal) return;
    await act(
      selected.profile.id,
      () => api.recordVerification(selected.profile.id, verificationDraft),
      `${verificationModal.title} result saved`,
    );
    setVerificationModal(null);
  }

  async function compareProfiles() {
    if (!compareLeft || !compareRight) return;
    try {
      setBusy("compare");
      const [left, right] = await Promise.all([
        api.fingerprintLatest(compareLeft),
        api.fingerprintLatest(compareRight),
      ]);
      setCompareSnapshots({ left, right });
    } catch (err) {
      setError(errorText(err));
    } finally {
      setBusy(null);
    }
  }

  const comparison = useMemo(() => {
    const left = compareSnapshots.left;
    const right = compareSnapshots.right;
    if (!left || !right) return null;
    const rightMap = new Map(right.surfaces.filter((surface) => surface.stable).map((surface) => [surface.key, surface]));
    const rows = left.surfaces
      .filter((surface) => surface.stable)
      .map((surface) => {
        const other = rightMap.get(surface.key);
        return {
          key: surface.key,
          label: surface.label,
          category: surface.category,
          left: surface.value,
          right: other?.value ?? "Not observed",
          same: other?.value === surface.value,
        };
      });
    const matched = rows.filter((row) => row.same).length;
    return {
      rows,
      matched,
      total: rows.length,
      percent: rows.length ? Math.round((matched / rows.length) * 100) : 0,
    };
  }, [compareSnapshots]);

  const commandItems = useMemo(() => {
    const base = NAV.map((item) => ({
      id: `page-${item.id}`,
      title: item.label,
      hint: "Navigate",
      action: () => {
        setPage(item.id);
        setCommandOpen(false);
      },
    }));
    base.unshift({
      id: "new-profile",
      title: "Create new profile",
      hint: "Action",
      action: () => {
        openCreate();
        setCommandOpen(false);
      },
    });
    return base.filter((item) =>
      `${item.title} ${item.hint}`.toLowerCase().includes(commandQuery.toLowerCase()),
    );
  }, [commandQuery]);

  function renderProfileSelector(label: string) {
    if (!selected) return null;
    return (
      <div className="cx-profile-selector">
        <div className="cx-avatar large">{selected.profile.name.slice(0, 1).toUpperCase()}</div>
        <div>
          <span>{label}</span>
          <select value={selected.profile.id} onChange={(event) => setSelectedId(event.target.value)}>
            {profiles.map((item) => (
              <option key={item.profile.id} value={item.profile.id}>
                {item.profile.name}
              </option>
            ))}
          </select>
        </div>
      </div>
    );
  }

  function renderOverview() {
    const recent = profiles.slice(0, 5);
    return (
      <div className="cx-stack">
        <section className="cx-hero">
          <div className="cx-hero-copy">
            <span className="cx-eyebrow">Commercial privacy operations</span>
            <h2>Know what every profile is doing.</h2>
            <p>
              Isolation, policy enforcement, fingerprint stability and remote verification are shown separately so a green proxy check is never mistaken for proof of zero leakage.
            </p>
            <div className="cx-hero-actions">
              <button className="cx-button primary" type="button" onClick={openCreate}>＋ New profile</button>
              <button className="cx-button" type="button" onClick={() => setPage("verification")}>Open verification</button>
            </div>
          </div>
          <div className={`cx-system-orb ${status?.chromium_ready ? "ok" : "bad"}`}>
            <span>{status?.chromium_ready ? "✓" : "!"}</span>
            <strong>Chromium</strong>
            <small>{status?.chromium_ready ? "Ready" : "Needs attention"}</small>
          </div>
        </section>

        <section className="cx-metrics">
          <Metric label="Profiles" value={profiles.length} detail={`${runningCount} running`} tone="accent" />
          <Metric label="Verified" value={verifiedCount} detail={`${profiles.length - verifiedCount} need review`} />
          <Metric label="Fingerprint stable" value={stableCount} detail={`${profiles.length - stableCount} pending/drift`} />
          <Metric label="Critical" value={criticalCount} detail={criticalCount ? "Action required" : "No critical journal result"} tone={criticalCount ? "danger" : "good"} />
        </section>

        <div className="cx-grid two">
          <section className="cx-panel">
            <PanelTitle eyebrow="Recent" title="Profile health" action={<button className="cx-link" onClick={() => setPage("profiles")}>View all</button>} />
            {recent.length ? (
              <div className="cx-list">
                {recent.map((item) => {
                  const health = profileHealth(item);
                  return (
                    <button className="cx-profile-row" key={item.profile.id} type="button" onClick={() => focusProfile(item, "privacy")}>
                      <div className="cx-avatar">{item.profile.name.slice(0, 1).toUpperCase()}</div>
                      <div className="cx-grow">
                        <strong>{item.profile.name}</strong>
                        <span>{routeLabel(item)} · {fingerprintLabel(item)}</span>
                      </div>
                      <span className={`cx-badge ${health.state}`}>{health.label}</span>
                      <span className={`cx-runtime ${item.runtime.running ? "running" : ""}`}>{item.runtime.running ? "Running" : "Stopped"}</span>
                    </button>
                  );
                })}
              </div>
            ) : (
              <Empty title="No profiles yet" detail="Create your first isolated browser workspace." action={openCreate} />
            )}
          </section>

          <section className="cx-panel">
            <PanelTitle eyebrow="Runtime" title="System readiness" action={<button className="cx-link" onClick={() => setPage("diagnostics")}>Diagnostics</button>} />
            <div className="cx-health-list">
              {diagnostics.slice(0, 7).map((item) => (
                <div className="cx-health-row" key={item.id}>
                  <span className={`cx-health-dot ${item.status}`} />
                  <div>
                    <strong>{item.label}</strong>
                    <span>{item.detail}</span>
                  </div>
                </div>
              ))}
            </div>
          </section>
        </div>
      </div>
    );
  }

  function renderProfiles() {
    return (
      <div className="cx-stack">
        <section className="cx-toolbar">
          <div className="cx-search"><span>⌕</span><input value={query} onChange={(event) => setQuery(event.target.value)} placeholder="Search profiles, tags, proxy, verification…" /></div>
          <button className="cx-button" type="button" onClick={() => void refresh()}>↻ Refresh</button>
          <button className="cx-button primary" type="button" onClick={openCreate}>＋ New profile</button>
        </section>
        {loading ? <div className="cx-loading">Loading profiles…</div> : filteredProfiles.length ? (
          <section className="cx-profile-grid">
            {filteredProfiles.map((item) => {
              const health = profileHealth(item);
              const itemBusy = busy === item.profile.id;
              return (
                <article className="cx-profile-card" key={item.profile.id}>
                  <div className="cx-card-head">
                    <div className="cx-avatar large">{item.profile.name.slice(0, 1).toUpperCase()}</div>
                    <div className="cx-grow">
                      <div className="cx-name-line"><h3>{item.profile.name}</h3><span className={`cx-badge ${health.state}`}>{health.label}</span></div>
                      <p>{item.profile.notes || "No notes"}</p>
                    </div>
                  </div>
                  <div className="cx-fact-grid">
                    <Fact label="Network" value={routeLabel(item)} />
                    <Fact label="Privacy" value={`${item.profile.privacy.preset} · ${item.profile.privacy.network_guard}`} />
                    <Fact label="Fingerprint" value={fingerprintLabel(item)} tone={item.fingerprint.state === "drift" ? "danger" : undefined} />
                    <Fact label="Verification" value={verificationLabel(item.verification.state)} tone={item.verification.state === "critical" ? "danger" : undefined} />
                  </div>
                  {item.profile.tags.length > 0 && <div className="cx-tags">{item.profile.tags.map((tag) => <span key={tag}>{tag}</span>)}</div>}
                  <div className="cx-card-actions">
                    {item.runtime.running ? (
                      <button className="cx-button stop" type="button" disabled={itemBusy} onClick={() => void act(item.profile.id, () => api.stopProfile(item.profile.id), "Profile stopped")}>■ Stop</button>
                    ) : (
                      <button className="cx-button primary" type="button" disabled={itemBusy || !status?.chromium_ready} onClick={() => void act(item.profile.id, () => api.launchProfile(item.profile.id), "Profile launched")}>▶ Launch</button>
                    )}
                    <button className="cx-button" type="button" onClick={() => focusProfile(item, "privacy")}>Privacy</button>
                    <button className="cx-button" type="button" onClick={() => focusProfile(item, "fingerprints")}>Fingerprint</button>
                    <button className="cx-icon-button" title="Edit" type="button" onClick={() => openEdit(item)}>✎</button>
                  </div>
                  <div className="cx-card-foot">
                    <span>Updated {formatRelative(item.profile.updated_at)}</span>
                    <div><button disabled={item.runtime.running} onClick={() => void resetProfile(item)}>Reset data</button><button className="danger" disabled={item.runtime.running} onClick={() => void deleteProfile(item)}>Delete</button></div>
                  </div>
                </article>
              );
            })}
          </section>
        ) : <Empty title="No matching profiles" detail="Try another search or create a profile." action={profiles.length ? undefined : openCreate} />}
      </div>
    );
  }

  function renderPrivacy() {
    if (!selected) return <Empty title="No profile selected" detail="Create a profile to configure privacy." action={openCreate} />;
    const state = privacyStatus?.overall_status ?? "verify_external";
    return (
      <div className="cx-stack">
        <section className="cx-context-bar">
          {renderProfileSelector("Privacy workspace")}
          <div className="cx-context-actions">
            <button className="cx-button" type="button" onClick={() => openEdit(selected)}>Edit policy</button>
            <button className="cx-button" type="button" disabled={busy === selected.profile.id} onClick={() => void loadSelectedDetail(selected.profile.id)}>↻ Preflight</button>
          </div>
        </section>

        <section className={`cx-health-hero ${state}`}>
          <div className="cx-health-symbol">◈</div>
          <div className="cx-grow">
            <span className="cx-eyebrow">{state.replaceAll("_", " ")}</span>
            <h2>{selected.profile.name}</h2>
            <p>{privacyStatus?.message ?? "Run preflight to evaluate the local policy and verification state."}</p>
            <div className="cx-inline-meta">
              <span>{selected.profile.privacy.preset} preset</span>
              <span>{selected.profile.privacy.network_guard} guard</span>
              <span>WebRTC {selected.profile.privacy.webrtc}</span>
              <span>Verified {formatRelative(selected.verification.last_verified_at)}</span>
            </div>
          </div>
          {!selected.runtime.running ? <button className="cx-button primary" disabled={!status?.chromium_ready || busy === selected.profile.id} onClick={() => void act(selected.profile.id, () => api.launchProfile(selected.profile.id), "Launched with privacy policy")}>▶ Launch with policy</button> : <span className="cx-badge healthy">Running</span>}
        </section>

        <section className="cx-health-matrix">
          <HealthCell title="Policy" value={privacyStatus?.policy_applied.applied ? "Applied" : "Restart needed"} state={privacyStatus?.policy_applied.applied ? "healthy" : "review"} detail={privacyStatus?.policy_applied.message ?? "Not checked"} />
          <HealthCell title="Network guard" value={selected.profile.privacy.network_guard} state={privacyStatus?.network_probe.reachable === false ? "critical" : "healthy"} detail={privacyStatus?.network_probe.message ?? "Run preflight"} />
          <HealthCell title="Fingerprint" value={fingerprintLabel(selected)} state={selected.fingerprint.state === "drift" ? "review" : selected.fingerprint.state === "stable" ? "healthy" : "pending"} detail={`${selected.fingerprint.surface_count} observed surfaces`} />
          <HealthCell title="Remote verification" value={verificationLabel(selected.verification.state)} state={selected.verification.state} detail={`${selected.verification.latest_test_count} current test results`} />
        </section>

        <div className="cx-grid two">
          <section className="cx-panel">
            <PanelTitle eyebrow="Enforcement" title="Per-profile privacy policy" />
            <div className="cx-policy-list">
              <Policy label="Third-party cookies" value={selected.profile.privacy.block_third_party_cookies ? "Blocked" : "Default"} />
              <Policy label="WebRTC non-proxied UDP" value={selected.profile.privacy.webrtc === "proxied_only" ? "Disabled" : "Default"} />
              <Policy label="Notifications" value={selected.profile.privacy.block_notifications ? "Blocked" : "Default"} />
              <Policy label="Geolocation" value={selected.profile.privacy.block_geolocation ? "Blocked" : "Default"} />
              <Policy label="Camera" value={selected.profile.privacy.block_camera ? "Blocked" : "Default"} />
              <Policy label="Microphone" value={selected.profile.privacy.block_microphone ? "Blocked" : "Default"} />
            </div>
          </section>
          <section className="cx-panel">
            <PanelTitle eyebrow="Route" title="Network preflight" action={<button className="cx-link" onClick={() => void probe(selected)}>Run again</button>} />
            <div className="cx-route-card">
              <span className={`cx-route-light ${privacyStatus?.network_probe.reachable === false ? "bad" : "good"}`} />
              <div><strong>{routeLabel(selected)}</strong><p>{privacyStatus?.network_probe.message ?? "Preflight not run yet."}</p></div>
            </div>
            <div className="cx-notice">Endpoint reachability is only a local preflight. Remote IP, DNS, IPv6 and WebRTC results stay separate and must be recorded in Verification.</div>
          </section>
        </div>

        <section className="cx-panel">
          <PanelTitle eyebrow="External truth" title="Verification shortcuts" action={<button className="cx-link" onClick={() => setPage("verification")}>Full verification center</button>} />
          <div className="cx-test-grid compact">
            {TESTS.filter((test) => test.core).map((test) => <TestCard key={test.id} test={test} onOpen={() => void openTest(test)} onRecord={() => openResult(test)} />)}
          </div>
        </section>
      </div>
    );
  }

  function renderFingerprints() {
    if (!selected) return <Empty title="No profile selected" detail="Create a profile before fingerprint auditing." action={openCreate} />;
    return (
      <div className="cx-stack">
        <section className="cx-context-bar">
          {renderProfileSelector("Fingerprint workspace")}
          <div className="cx-context-actions">
            <button className="cx-button" disabled={!selected.fingerprint.snapshot_count || busy === selected.profile.id} onClick={() => { if (window.confirm("Use the latest snapshot as the new baseline after reviewing an intentional environment change?")) void act(selected.profile.id, () => api.setFingerprintBaseline(selected.profile.id), "Baseline updated"); }}>Set latest baseline</button>
            <button className="cx-button primary" disabled={!status?.chromium_ready || busy === selected.profile.id} onClick={() => void act(selected.profile.id, () => api.openPrivacyAudit(selected.profile.id), "Local audit opened")}>◇ Run local audit</button>
          </div>
        </section>

        <section className={`cx-fingerprint-hero ${selected.fingerprint.state}`}>
          <div className="cx-score-ring"><strong>{selected.fingerprint.consistency_score ?? "—"}</strong><span>/100</span></div>
          <div className="cx-grow"><span className="cx-eyebrow">Per-profile baseline</span><h2>{selected.profile.name}</h2><p>{selected.fingerprint.state === "stable" ? "Stable surfaces match the current baseline." : selected.fingerprint.state === "drift" ? `${selected.fingerprint.drift_count} stable surfaces changed since baseline.` : "Run an audit to build or review this profile's browser-visible baseline."}</p><div className="cx-inline-meta"><span>{selected.fingerprint.snapshot_count} snapshots</span><span>{selected.fingerprint.surface_count} surfaces</span><span>{selected.fingerprint.issue_count} review items</span><span>Last audit {formatRelative(selected.fingerprint.last_captured_at)}</span></div></div>
        </section>

        <div className="cx-grid two">
          <section className="cx-panel">
            <PanelTitle eyebrow="Timeline" title="Fingerprint history" />
            {fingerprintHistory.length ? <div className="cx-timeline">{fingerprintHistory.map((entry) => <div className="cx-timeline-row" key={entry.snapshot_id}><span className={`cx-score-mini ${entry.drift_count ? "bad" : entry.issue_count ? "warn" : "good"}`}>{entry.consistency_score}</span><div className="cx-grow"><strong>{formatDate(entry.captured_at)}</strong><span>{entry.surface_count} surfaces · {entry.drift_count} drift · {entry.issue_count} review</span></div></div>)}</div> : <Empty title="No fingerprint history" detail="Run the local audit to create the first profile baseline." />}
          </section>
          <section className="cx-panel">
            <PanelTitle eyebrow="Latest" title="Drift & consistency" />
            {!fingerprintLatest ? <Empty title="Waiting for first snapshot" detail="The latest snapshot will be compared with the profile baseline." /> : <div className="cx-drift-list">{fingerprintLatest.drift.length ? fingerprintLatest.drift.map((item) => <div className="cx-drift-row" key={item.key}><div><span>{item.category}</span><strong>{item.label}</strong></div><code>{item.baseline_value}</code><b>→</b><code>{item.current_value}</code></div>) : <div className="cx-success-callout"><span>✓</span><div><strong>No stable-surface drift</strong><p>The latest stable observations match the current baseline.</p></div></div>}{fingerprintLatest.issues.map((issue) => <div className="cx-warning-callout" key={issue}>! {issue}</div>)}</div>}
          </section>
        </div>

        <section className="cx-panel">
          <PanelTitle eyebrow="Correlation review" title="Compare two profiles" />
          <div className="cx-compare-controls">
            <select value={compareLeft ?? ""} onChange={(event) => setCompareLeft(event.target.value)}>{profiles.map((item) => <option key={item.profile.id} value={item.profile.id}>{item.profile.name}</option>)}</select>
            <span>vs</span>
            <select value={compareRight ?? ""} onChange={(event) => setCompareRight(event.target.value)}>{profiles.map((item) => <option key={item.profile.id} value={item.profile.id}>{item.profile.name}</option>)}</select>
            <button className="cx-button" disabled={busy === "compare"} onClick={() => void compareProfiles()}>Compare stable surfaces</button>
          </div>
          {comparison && <div className="cx-comparison-result"><div className="cx-comparison-score"><strong>{comparison.percent}%</strong><span>{comparison.matched}/{comparison.total} stable surfaces equal</span></div><div className="cx-comparison-table">{comparison.rows.slice(0, 16).map((row) => <div className="cx-comparison-row" key={row.key}><div><span>{row.category}</span><strong>{row.label}</strong></div><code>{row.left}</code><span className={`cx-equality ${row.same ? "same" : "different"}`}>{row.same ? "Same" : "Different"}</span><code>{row.right}</code></div>)}</div><p className="cx-footnote">Similarity is a privacy diagnostic, not a claim about whether a third party will or will not correlate profiles.</p></div>}
        </section>
      </div>
    );
  }

  function renderVerification() {
    if (!selected) return <Empty title="No profile selected" detail="Create a profile before remote verification." action={openCreate} />;
    return (
      <div className="cx-stack">
        <section className="cx-context-bar">
          {renderProfileSelector("Verification workspace")}
          <div className="cx-context-actions"><span className={`cx-badge ${selected.verification.state}`}>{verificationLabel(selected.verification.state)}</span><button className="cx-button" onClick={() => void loadSelectedDetail(selected.profile.id)}>↻ Refresh journal</button></div>
        </section>

        <section className="cx-verification-summary">
          <Metric label="Current tests" value={selected.verification.latest_test_count} detail="latest result per test" />
          <Metric label="Pass" value={selected.verification.pass_count} detail="current passing checks" tone="good" />
          <Metric label="Warnings" value={selected.verification.warning_count + selected.verification.inconclusive_count} detail="review/inconclusive" />
          <Metric label="Critical" value={selected.verification.critical_count} detail="unexpected exposure" tone={selected.verification.critical_count ? "danger" : "good"} />
        </section>

        <section className="cx-panel">
          <PanelTitle eyebrow="Remote website tests" title="Verification lab" />
          <div className="cx-test-sections">
            <div><h3>Network exposure</h3><div className="cx-test-grid">{TESTS.filter((test) => test.group === "network").map((test) => <TestCard key={test.id} test={test} onOpen={() => void openTest(test)} onRecord={() => openResult(test)} />)}</div></div>
            <div><h3>Fingerprint perspective</h3><div className="cx-test-grid">{TESTS.filter((test) => test.group === "fingerprint").map((test) => <TestCard key={test.id} test={test} onOpen={() => void openTest(test)} onRecord={() => openResult(test)} />)}</div></div>
          </div>
        </section>

        <section className="cx-panel">
          <PanelTitle eyebrow="Audit trail" title="Verification journal" />
          {verificationHistory.length ? <div className="cx-verification-history">{verificationHistory.map((record) => { const test = TESTS.find((item) => item.id === record.test); return <div className="cx-verification-row" key={record.id}><span className={`cx-result ${record.result}`}>{resultLabel(record.result)}</span><div className="cx-grow"><strong>{test?.title ?? record.test}</strong><span>{formatDate(record.verified_at)} · policy v{record.policy_version}</span>{(record.observed || record.expected) && <small>Expected: {record.expected || "—"} · Observed: {record.observed || "—"}</small>}{record.notes && <p>{record.notes}</p>}</div></div>; })}</div> : <Empty title="No external verification recorded" detail="Open a test in this exact profile, review the result, then record Pass/Warning/Critical/Inconclusive." />}
        </section>
      </div>
    );
  }

  function renderNetwork() {
    return (
      <div className="cx-stack">
        <section className="cx-info-banner"><span>⌁</span><div><strong>Fail closed where Dravyn can prove it locally</strong><p>Strict proxy profiles block launch when the configured endpoint is unreachable. Public IP, DNS, IPv6 and WebRTC remain remote-verification concerns and are never inferred from TCP reachability alone.</p></div></section>
        <section className="cx-panel">
          <PanelTitle eyebrow="Routes" title="Profile network policies" />
          {profiles.length ? <div className="cx-network-list">{profiles.map((item) => { const result = probes[item.profile.id]; return <div className="cx-network-row" key={item.profile.id}><div className="cx-avatar">{item.profile.name.slice(0, 1).toUpperCase()}</div><div className="cx-grow"><strong>{item.profile.name}</strong><span>{routeLabel(item)}</span></div><span className="cx-chip">Guard: {item.profile.privacy.network_guard}</span>{result && <span className={`cx-badge ${result.reachable === false ? "critical" : "healthy"}`}>{result.reachable === false ? "Unreachable" : result.reachable === true ? `${result.latency_ms ?? 0} ms` : "Direct"}</span>}<button className="cx-button" disabled={busy === item.profile.id} onClick={() => void probe(item)}>Preflight</button><button className="cx-link" onClick={() => focusProfile(item, "verification")}>Verify</button></div>})}</div> : <Empty title="No profiles" detail="Create a profile to configure network routing." action={openCreate} />}
        </section>
      </div>
    );
  }

  function renderDiagnostics() {
    return (
      <section className="cx-panel">
        <PanelTitle eyebrow="Runtime" title="System diagnostics" action={<button className="cx-button" onClick={() => void refresh()}>↻ Run again</button>} />
        <div className="cx-diagnostic-grid">
          {diagnostics.map((item) => <article className="cx-diagnostic" key={item.id}><div className={`cx-diagnostic-icon ${item.status}`}>{item.status === "ok" ? "✓" : item.status === "warning" ? "!" : "×"}</div><div><span>{item.status}</span><h3>{item.label}</h3><p>{item.detail}</p></div></article>)}
        </div>
      </section>
    );
  }

  function renderSettings() {
    return (
      <div className="cx-grid two">
        <section className="cx-panel"><PanelTitle eyebrow="Workspace" title="Runtime paths" /><Settings label="Dravyn home" value={status?.workspace ?? "Checking…"} /><Settings label="Chromium" value={status?.browser_binary ?? "Checking…"} /><Settings label="Chromium state" value={status?.chromium_state ?? "Checking…"} /><Settings label="Fingerprint capture" value={status?.fingerprint_capture_origin ?? "Checking…"} /><Settings label="Verification journal" value={status?.verification_store ?? "Checking…"} /></section>
        <section className="cx-panel"><PanelTitle eyebrow="Assurance model" title="What green means" /><div className="cx-settings-copy"><p><strong>Policy applied</strong> means Dravyn wrote and read back the selected Chromium preferences before launch.</p><p><strong>Network preflight</strong> means the configured proxy endpoint accepted a local TCP connection.</p><p><strong>Verified</strong> means the operator journal currently has no warning/critical result for the recorded remote tests; it is not a claim of anonymity or undetectability.</p><p><strong>Fingerprint stable</strong> means stable observed surfaces match that profile's baseline.</p></div></section>
      </div>
    );
  }

  const pageTitle: Record<Page, [string, string]> = {
    overview: ["Overview", "Privacy assurance at a glance"],
    profiles: ["Profiles", "Operate isolated Chromium workspaces"],
    privacy: ["Privacy Center", "Policy enforcement and health"],
    fingerprints: ["Fingerprint Center", "Baseline, drift and cross-profile visibility"],
    verification: ["Verification Center", "Record the remote website perspective"],
    network: ["Network", "Routes, guards and preflight"],
    diagnostics: ["Diagnostics", "Runtime and storage readiness"],
    settings: ["Settings", "Workspace and assurance model"],
  };

  const [title, subtitle] = pageTitle[page];

  return (
    <div className="cx-app">
      <aside className="cx-sidebar">
        <button className="cx-brand" type="button" onClick={() => setPage("overview")}><span>D</span><div><strong>Dravyn</strong><small>Privacy Browser Core</small></div></button>
        <div className="cx-nav-scroll">
          {(["operate", "assure", "system"] as const).map((section) => <div className="cx-nav-group" key={section}><label>{section === "operate" ? "Operate" : section === "assure" ? "Assure" : "System"}</label>{NAV.filter((item) => item.section === section).map((item) => <button key={item.id} className={page === item.id ? "active" : ""} onClick={() => setPage(item.id)}><span>{item.icon}</span>{item.label}{item.id === "profiles" && profiles.length > 0 && <em>{profiles.length}</em>}{item.id === "verification" && criticalCount > 0 && <em className="alert">{criticalCount}</em>}</button>)}</div>)}
        </div>
        <div className="cx-sidebar-foot">
          <div className={`cx-browser-status ${status?.chromium_ready ? "ok" : "bad"}`}><span /><div><strong>Chromium {status?.chromium_ready ? "ready" : "not ready"}</strong><small>{status?.chromium_state ?? "Checking…"}</small></div></div>
          <button className="cx-command-hint" type="button" onClick={() => setCommandOpen(true)}><span>⌘</span> Command palette <kbd>Ctrl K</kbd></button>
          <small>M6 · v{status?.version ?? "0.1.0"}</small>
        </div>
      </aside>

      <main className="cx-main">
        <header className="cx-topbar">
          <div><span className="cx-eyebrow">M6 · Commercial privacy operations</span><h1>{title}</h1><p>{subtitle}</p></div>
          <div className="cx-top-actions"><button className="cx-command" type="button" onClick={() => setCommandOpen(true)}>⌕ Search or command <kbd>Ctrl K</kbd></button><button className="cx-button primary" type="button" onClick={openCreate}>＋ New profile</button></div>
        </header>
        {error && <div className="cx-error"><span>!</span><div><strong>Dravyn needs attention</strong><p>{error}</p></div><button onClick={() => setError("")}>×</button></div>}
        <div className="cx-content">
          {page === "overview" && renderOverview()}
          {page === "profiles" && renderProfiles()}
          {page === "privacy" && renderPrivacy()}
          {page === "fingerprints" && renderFingerprints()}
          {page === "verification" && renderVerification()}
          {page === "network" && renderNetwork()}
          {page === "diagnostics" && renderDiagnostics()}
          {page === "settings" && renderSettings()}
        </div>
      </main>

      {toast && <div className="cx-toast">✓ {toast}</div>}

      {editorOpen && <div className="cx-modal-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setEditorOpen(false); }}><form className="cx-modal profile-editor" onSubmit={(event) => void saveProfile(event)}><div className="cx-modal-head"><div><span className="cx-eyebrow">Profile workspace</span><h2>{editingId ? "Edit profile" : "Create profile"}</h2><p>Configure browser workspace, route and defensive privacy policy together.</p></div><button type="button" onClick={() => setEditorOpen(false)}>×</button></div><div className="cx-form-sections"><section><h3>Identity</h3><div className="cx-form-grid"><label className="full"><span>Name</span><input required maxLength={80} value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} placeholder="Work US" /></label><label className="full"><span>Notes</span><textarea rows={3} value={draft.notes} onChange={(event) => setDraft({ ...draft, notes: event.target.value })} placeholder="Purpose or workflow" /></label><label className="full"><span>Tags</span><input value={tagsText} onChange={(event) => setTagsText(event.target.value)} placeholder="work, qa, client" /></label></div></section><section><h3>Browser</h3><div className="cx-form-grid"><label className="full"><span>Start URL</span><input value={draft.browser.start_url ?? ""} onChange={(event) => setDraft({ ...draft, browser: { ...draft.browser, start_url: event.target.value || null } })} /></label><label><span>Width</span><input type="number" min={640} max={7680} value={draft.browser.window_width ?? ""} onChange={(event) => setDraft({ ...draft, browser: { ...draft.browser, window_width: event.target.value ? Number(event.target.value) : null } })} /></label><label><span>Height</span><input type="number" min={480} max={4320} value={draft.browser.window_height ?? ""} onChange={(event) => setDraft({ ...draft, browser: { ...draft.browser, window_height: event.target.value ? Number(event.target.value) : null } })} /></label></div></section><section><h3>Network</h3><div className="cx-form-grid"><label className="full"><span>Route</span><select value={draft.network.mode} onChange={(event) => { const mode = event.target.value as NetworkMode; setDraft({ ...draft, network: mode === "direct" ? { mode, proxy: null } : { mode, proxy: draft.network.proxy ?? { scheme: "http", host: "127.0.0.1", port: 8080 } } }); }}><option value="direct">Direct connection</option><option value="proxy">Explicit proxy</option></select></label>{draft.network.mode === "proxy" && draft.network.proxy && <><label><span>Scheme</span><select value={draft.network.proxy.scheme} onChange={(event) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, scheme: event.target.value as ProxyScheme } } })}><option value="http">HTTP</option><option value="https">HTTPS</option><option value="socks5">SOCKS5</option></select></label><label><span>Port</span><input type="number" min={1} max={65535} value={draft.network.proxy.port} onChange={(event) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, port: Number(event.target.value) } } })} /></label><label className="full"><span>Host</span><input value={draft.network.proxy.host} onChange={(event) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, host: event.target.value } } })} /></label></>}</div></section><section><h3>Privacy policy</h3><div className="cx-form-grid"><label className="full"><span>Preset</span><select value={draft.privacy.preset} onChange={(event) => setDraft({ ...draft, privacy: presetPolicy(event.target.value as PrivacyPreset, draft.privacy) })}><option value="standard">Standard</option><option value="balanced">Balanced</option><option value="strict">Strict</option><option value="custom">Custom</option></select></label><label><span>Network guard</span><select value={draft.privacy.network_guard} onChange={(event) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", network_guard: event.target.value as ProfileDraft["privacy"]["network_guard"] } })}><option value="off">Off</option><option value="monitor">Monitor</option><option value="strict">Strict · fail closed</option></select></label><label><span>WebRTC</span><select value={draft.privacy.webrtc} onChange={(event) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", webrtc: event.target.value as ProfileDraft["privacy"]["webrtc"] } })}><option value="default">Default</option><option value="proxied_only">Disable non-proxied UDP</option></select></label><div className="cx-toggle-grid full">{([ ["block_third_party_cookies", "Block third-party cookies"], ["block_notifications", "Block notifications"], ["block_geolocation", "Block geolocation"], ["block_camera", "Block camera"], ["block_microphone", "Block microphone"] ] as const).map(([key, label]) => <label className="cx-toggle" key={key}><input type="checkbox" checked={draft.privacy[key]} onChange={(event) => setDraft({ ...draft, privacy: { ...draft.privacy, preset: "custom", [key]: event.target.checked } })} /><span /><strong>{label}</strong></label>)}</div></div></section></div><div className="cx-modal-actions"><button className="cx-button" type="button" onClick={() => setEditorOpen(false)}>Cancel</button><button className="cx-button primary" type="submit" disabled={busy === (editingId ?? "new")}>{editingId ? "Save changes" : "Create profile"}</button></div></form></div>}

      {verificationModal && selected && <div className="cx-modal-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setVerificationModal(null); }}><form className="cx-modal verification-editor" onSubmit={(event) => void saveVerification(event)}><div className="cx-modal-head"><div><span className="cx-eyebrow">Verification journal</span><h2>{verificationModal.title}</h2><p>Record what the external website showed for {selected.profile.name}. Do not mark Pass unless you reviewed the actual remote result.</p></div><button type="button" onClick={() => setVerificationModal(null)}>×</button></div><div className="cx-form-grid"><label className="full"><span>Result</span><div className="cx-result-picker">{(["pass", "warning", "critical", "inconclusive"] as VerificationResult[]).map((result) => <button key={result} type="button" className={`${result} ${verificationDraft.result === result ? "active" : ""}`} onClick={() => setVerificationDraft({ ...verificationDraft, result })}>{resultLabel(result)}</button>)}</div></label><label><span>Expected</span><input value={verificationDraft.expected ?? ""} onChange={(event) => setVerificationDraft({ ...verificationDraft, expected: event.target.value || null })} placeholder="Proxy IP / expected resolver" /></label><label><span>Observed</span><input value={verificationDraft.observed ?? ""} onChange={(event) => setVerificationDraft({ ...verificationDraft, observed: event.target.value || null })} placeholder="Value shown by test" /></label><label className="full"><span>Notes</span><textarea rows={4} value={verificationDraft.notes} onChange={(event) => setVerificationDraft({ ...verificationDraft, notes: event.target.value })} placeholder="What was checked and why this result was selected" /></label></div><div className="cx-modal-actions"><button className="cx-button" type="button" onClick={() => setVerificationModal(null)}>Cancel</button><button className="cx-button primary" type="submit">Save verification result</button></div></form></div>}

      {commandOpen && <div className="cx-command-backdrop" onMouseDown={(event) => { if (event.target === event.currentTarget) setCommandOpen(false); }}><div className="cx-command-palette"><div className="cx-command-input"><span>⌕</span><input autoFocus value={commandQuery} onChange={(event) => setCommandQuery(event.target.value)} placeholder="Go to page or run an action…" /><kbd>Esc</kbd></div><div className="cx-command-results">{commandItems.map((item) => <button key={item.id} onClick={item.action}><span>{item.title}</span><small>{item.hint}</small></button>)}</div></div></div>}
    </div>
  );
}

function Metric({ label, value, detail, tone }: { label: string; value: number | string; detail: string; tone?: string }) {
  return <article className={`cx-metric ${tone ?? ""}`}><span>{label}</span><strong>{value}</strong><small>{detail}</small></article>;
}

function PanelTitle({ eyebrow, title, action }: { eyebrow: string; title: string; action?: React.ReactNode }) {
  return <div className="cx-panel-title"><div><span className="cx-eyebrow">{eyebrow}</span><h2>{title}</h2></div>{action}</div>;
}

function Fact({ label, value, tone }: { label: string; value: string; tone?: string }) {
  return <div className={`cx-fact ${tone ?? ""}`}><span>{label}</span><strong>{value}</strong></div>;
}

function Policy({ label, value }: { label: string; value: string }) {
  return <div className="cx-policy"><span>{label}</span><strong>{value}</strong></div>;
}

function HealthCell({ title, value, state, detail }: { title: string; value: string; state: string; detail: string }) {
  return <article className={`cx-health-cell ${state}`}><span>{title}</span><strong>{value}</strong><small>{detail}</small></article>;
}

function TestCard({ test, onOpen, onRecord }: { test: TestDefinition; onOpen: () => void; onRecord: () => void }) {
  return <article className="cx-test-card"><div className="cx-test-icon">↗</div><div><strong>{test.title}</strong><p>{test.subtitle}</p>{test.core && <span className="cx-core-tag">Core verification</span>}</div><div className="cx-test-actions"><button className="cx-button" type="button" onClick={onOpen}>Open test</button><button className="cx-link" type="button" onClick={onRecord}>Record result</button></div></article>;
}

function Settings({ label, value }: { label: string; value: string }) {
  return <div className="cx-setting"><span>{label}</span><code>{value}</code></div>;
}

function Empty({ title, detail, action }: { title: string; detail: string; action?: () => void }) {
  return <div className="cx-empty"><span>◇</span><h3>{title}</h3><p>{detail}</p>{action && <button className="cx-button primary" onClick={action}>＋ New profile</button>}</div>;
}
