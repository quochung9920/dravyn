export type NetworkMode = "direct" | "proxy";
export type ProxyScheme = "http" | "https" | "socks5";
export type DiagnosticStatus = "ok" | "warning" | "error";
export type FingerprintState = "not_audited" | "stable" | "review" | "drift";
export type PrivacyPreset = "standard" | "balanced" | "strict" | "custom";
export type NetworkGuardMode = "off" | "monitor" | "strict";
export type WebRtcPolicy = "default" | "proxied_only";
export type ExternalVerificationTest =
  | "browserleaks_ip"
  | "browserleaks_webrtc"
  | "browserleaks_dns"
  | "browserleaks_canvas"
  | "browserleaks_webgl"
  | "eff"
  | "amiunique";

export interface ProxyConfig {
  scheme: ProxyScheme;
  host: string;
  port: number;
}

export interface NetworkConfig {
  mode: NetworkMode;
  proxy: ProxyConfig | null;
}

export interface PrivacyPolicy {
  preset: PrivacyPreset;
  network_guard: NetworkGuardMode;
  webrtc: WebRtcPolicy;
  block_third_party_cookies: boolean;
  block_notifications: boolean;
  block_geolocation: boolean;
  block_camera: boolean;
  block_microphone: boolean;
}

export interface BrowserConfig {
  start_url: string | null;
  window_width: number | null;
  window_height: number | null;
}

export interface Profile {
  id: string;
  name: string;
  notes: string;
  tags: string[];
  browser: BrowserConfig;
  network: NetworkConfig;
  privacy: PrivacyPolicy;
  created_at: number;
  updated_at: number;
}

export interface RuntimeStatus {
  running: boolean;
  pid: number | null;
  started_at: number | null;
}

export interface FingerprintSummary {
  profile_id: string;
  baseline_present: boolean;
  snapshot_count: number;
  last_captured_at: number | null;
  consistency_score: number | null;
  drift_count: number;
  issue_count: number;
  surface_count: number;
  state: FingerprintState;
}

export interface FingerprintSurface {
  key: string;
  label: string;
  category: string;
  value: string;
  stable: boolean;
}

export interface FingerprintDriftItem {
  key: string;
  label: string;
  category: string;
  baseline_value: string;
  current_value: string;
}

export interface FingerprintSnapshot {
  id: string;
  profile_id: string;
  captured_at: number;
  consistency_score: number;
  surfaces: FingerprintSurface[];
  issues: string[];
  drift: FingerprintDriftItem[];
}

export interface FingerprintHistoryEntry {
  snapshot_id: string;
  captured_at: number;
  consistency_score: number;
  drift_count: number;
  issue_count: number;
  surface_count: number;
}

export interface ProfileView {
  profile: Profile;
  runtime: RuntimeStatus;
  fingerprint: FingerprintSummary;
}

export interface ProfileDraft {
  name: string;
  notes: string;
  tags: string[];
  browser: BrowserConfig;
  network: NetworkConfig;
  privacy: PrivacyPolicy;
}

export interface AppStatus {
  chromium_ready: boolean;
  chromium_state: string;
  browser_binary: string;
  workspace: string;
  version: string;
  fingerprint_capture_origin: string;
}

export interface NetworkProbe {
  mode: string;
  endpoint: string | null;
  valid: boolean;
  reachable: boolean | null;
  latency_ms: number | null;
  message: string;
}

export interface PrivacyAppliedStatus {
  preferences_path: string;
  preferences_present: boolean;
  applied: boolean;
  expected_webrtc_policy: string;
  actual_webrtc_policy: string | null;
  third_party_cookies_blocked: boolean;
  blocked_permission_count: number;
  message: string;
}

export interface PrivacyStatus {
  profile_id: string;
  preset: string;
  network_guard: string;
  webrtc_policy: string;
  policy_applied: PrivacyAppliedStatus;
  network_probe: NetworkProbe;
  overall_status: "critical" | "restart_required" | "verify_external";
  external_verification_required: boolean;
  message: string;
}

export interface DiagnosticItem {
  id: string;
  label: string;
  status: DiagnosticStatus;
  detail: string;
}
