import { FormEvent, useCallback, useEffect, useMemo, useState } from "react";
import { api } from "./api";
import type {
  AppStatus,
  NetworkMode,
  Profile,
  ProfileDraft,
  ProfileView,
  ProxyScheme,
} from "./types";

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
  };
}

function errorMessage(error: unknown) {
  return error instanceof Error ? error.message : String(error);
}

export default function App() {
  const [status, setStatus] = useState<AppStatus | null>(null);
  const [profiles, setProfiles] = useState<ProfileView[]>([]);
  const [loading, setLoading] = useState(true);
  const [busyId, setBusyId] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [editorOpen, setEditorOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [draft, setDraft] = useState<ProfileDraft>(emptyDraft());
  const [tagsText, setTagsText] = useState("");

  const refresh = useCallback(async () => {
    try {
      setError("");
      const [appStatus, rows] = await Promise.all([api.appStatus(), api.listProfiles()]);
      setStatus(appStatus);
      setProfiles(rows);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
    const timer = window.setInterval(() => void refresh(), 4000);
    return () => window.clearInterval(timer);
  }, [refresh]);

  const runningCount = useMemo(
    () => profiles.filter((item) => item.runtime.running).length,
    [profiles],
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
      if (editingId) {
        await api.updateProfile(editingId, payload);
      } else {
        await api.createProfile(payload);
      }
      setEditorOpen(false);
      await refresh();
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
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusyId(null);
    }
  }

  async function deleteProfile(item: ProfileView) {
    if (!window.confirm(`Delete ${item.profile.name} and all of its local browser data?`)) return;
    await runAction(item.profile.id, () => api.deleteProfile(item.profile.id));
  }

  async function resetProfile(item: ProfileView) {
    if (!window.confirm(`Reset cookies, cache, history and site storage for ${item.profile.name}?`)) return;
    await runAction(item.profile.id, () => api.resetProfile(item.profile.id));
  }

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand"><span className="brandMark">D</span><span>Dravyn</span></div>
        <nav>
          <button className="navItem active" type="button">Profiles</button>
          <button className="navItem" type="button" disabled title="Planned for a later milestone">Network</button>
          <button className="navItem" type="button" disabled title="Planned for a later milestone">Diagnostics</button>
        </nav>
        <div className="scopeNote">Local-first profile isolation and explicit network controls for QA, privacy testing and authorized automation.</div>
      </aside>

      <main className="content">
        <header className="topbar">
          <div>
            <p className="eyebrow">M2 · Desktop Profile Manager</p>
            <h1>Browser profiles</h1>
          </div>
          <div className="topActions">
            <span className={`statusPill ${status?.chromium_ready ? "ready" : "notReady"}`}>
              <span className="dot" /> Chromium {status?.chromium_ready ? "Ready" : "Not ready"}
            </span>
            <button className="primary" type="button" onClick={openCreate}>+ New profile</button>
          </div>
        </header>

        <section className="metrics" aria-label="Workspace summary">
          <article><span>Total profiles</span><strong>{profiles.length}</strong></article>
          <article><span>Running now</span><strong>{runningCount}</strong></article>
          <article className="wide"><span>Browser binary</span><code>{status?.browser_binary ?? "Checking…"}</code></article>
        </section>

        {error && <div className="errorBanner" role="alert">{error}</div>}

        <section className="panel">
          <div className="panelHeader">
            <div><h2>Profiles</h2><p>Each profile has its own Chromium user-data directory.</p></div>
            <button className="secondary" type="button" onClick={() => void refresh()}>Refresh</button>
          </div>

          {loading ? (
            <div className="empty">Loading profiles…</div>
          ) : profiles.length === 0 ? (
            <div className="empty">
              <div className="emptyIcon">◎</div>
              <h3>Create your first profile</h3>
              <p>Dravyn will keep its cookies, history, local storage and session data isolated from every other profile.</p>
              <button className="primary" type="button" onClick={openCreate}>Create profile</button>
            </div>
          ) : (
            <div className="profileList">
              {profiles.map((item) => {
                const busy = busyId === item.profile.id;
                const proxy = item.profile.network.proxy;
                return (
                  <article className="profileCard" key={item.profile.id}>
                    <div className="profileIdentity">
                      <div className="avatar">{item.profile.name.slice(0, 1).toUpperCase()}</div>
                      <div className="profileText">
                        <div className="nameRow"><h3>{item.profile.name}</h3><span className={`runBadge ${item.runtime.running ? "running" : "stopped"}`}>{item.runtime.running ? `Running · PID ${item.runtime.pid}` : "Stopped"}</span></div>
                        <p>{item.profile.notes || "No notes"}</p>
                        <div className="metaRow">
                          <span>{item.profile.network.mode === "direct" ? "Direct connection" : `${proxy?.scheme ?? "proxy"}://${proxy?.host ?? "?"}:${proxy?.port ?? "?"}`}</span>
                          <span>{item.profile.browser.window_width ?? "auto"} × {item.profile.browser.window_height ?? "auto"}</span>
                          {item.profile.tags.map((tag) => <span className="tag" key={tag}>{tag}</span>)}
                        </div>
                      </div>
                    </div>
                    <div className="cardActions">
                      <button type="button" className="secondary" disabled={busy} onClick={() => openEdit(item)}>Edit</button>
                      <button type="button" className="secondary" disabled={busy || item.runtime.running} onClick={() => void resetProfile(item)}>Reset data</button>
                      <button type="button" className="dangerGhost" disabled={busy || item.runtime.running} onClick={() => void deleteProfile(item)}>Delete</button>
                      {item.runtime.running ? (
                        <button type="button" className="stop" disabled={busy} onClick={() => void runAction(item.profile.id, () => api.stopProfile(item.profile.id))}>{busy ? "Stopping…" : "■ Stop"}</button>
                      ) : (
                        <button type="button" className="launch" disabled={busy || !status?.chromium_ready} onClick={() => void runAction(item.profile.id, () => api.launchProfile(item.profile.id))}>{busy ? "Launching…" : "▶ Launch"}</button>
                      )}
                    </div>
                  </article>
                );
              })}
            </div>
          )}
        </section>

        {editorOpen && (
          <div className="modalBackdrop" role="presentation" onMouseDown={(event) => { if (event.target === event.currentTarget) setEditorOpen(false); }}>
            <form className="modal" onSubmit={(event) => void saveProfile(event)}>
              <div className="modalHeader"><div><p className="eyebrow">Profile configuration</p><h2>{editingId ? "Edit profile" : "New profile"}</h2></div><button className="iconButton" type="button" onClick={() => setEditorOpen(false)} aria-label="Close">×</button></div>

              <div className="formGrid">
                <label className="full"><span>Name</span><input required maxLength={80} value={draft.name} onChange={(e) => setDraft({ ...draft, name: e.target.value })} placeholder="QA profile" /></label>
                <label className="full"><span>Notes</span><textarea maxLength={4000} rows={3} value={draft.notes} onChange={(e) => setDraft({ ...draft, notes: e.target.value })} placeholder="Purpose of this profile" /></label>
                <label className="full"><span>Tags <small>comma separated</small></span><input value={tagsText} onChange={(e) => setTagsText(e.target.value)} placeholder="client-a, regression" /></label>
                <label className="full"><span>Start URL</span><input value={draft.browser.start_url ?? ""} onChange={(e) => setDraft({ ...draft, browser: { ...draft.browser, start_url: e.target.value || null } })} placeholder="https://example.com" /></label>
                <label><span>Window width</span><input type="number" min={640} max={7680} value={draft.browser.window_width ?? ""} onChange={(e) => setDraft({ ...draft, browser: { ...draft.browser, window_width: e.target.value ? Number(e.target.value) : null } })} /></label>
                <label><span>Window height</span><input type="number" min={480} max={4320} value={draft.browser.window_height ?? ""} onChange={(e) => setDraft({ ...draft, browser: { ...draft.browser, window_height: e.target.value ? Number(e.target.value) : null } })} /></label>

                <label className="full"><span>Network</span><select value={draft.network.mode} onChange={(e) => {
                  const mode = e.target.value as NetworkMode;
                  setDraft({ ...draft, network: mode === "direct" ? { mode, proxy: null } : { mode, proxy: draft.network.proxy ?? { scheme: "http", host: "127.0.0.1", port: 8080 } } });
                }}><option value="direct">Direct connection</option><option value="proxy">Explicit proxy</option></select></label>

                {draft.network.mode === "proxy" && draft.network.proxy && <>
                  <label><span>Proxy scheme</span><select value={draft.network.proxy.scheme} onChange={(e) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, scheme: e.target.value as ProxyScheme } } })}><option value="http">HTTP</option><option value="https">HTTPS</option><option value="socks5">SOCKS5</option></select></label>
                  <label><span>Proxy port</span><input type="number" min={1} max={65535} required value={draft.network.proxy.port} onChange={(e) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, port: Number(e.target.value) } } })} /></label>
                  <label className="full"><span>Proxy host</span><input required value={draft.network.proxy.host} onChange={(e) => setDraft({ ...draft, network: { ...draft.network, proxy: { ...draft.network.proxy!, host: e.target.value } } })} placeholder="127.0.0.1" /></label>
                  <p className="fieldHint full">M2 intentionally stores no proxy credentials and does not implement fingerprint spoofing or anti-fraud bypass features.</p>
                </>}
              </div>

              <div className="modalActions"><button className="secondary" type="button" onClick={() => setEditorOpen(false)}>Cancel</button><button className="primary" type="submit">{editingId ? "Save changes" : "Create profile"}</button></div>
            </form>
          </div>
        )}
      </main>
    </div>
  );
}
