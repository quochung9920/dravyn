export type NetworkMode = "direct" | "proxy";
export type ProxyScheme = "http" | "https" | "socks5";

export interface ProxyConfig {
  scheme: ProxyScheme;
  host: string;
  port: number;
}

export interface NetworkConfig {
  mode: NetworkMode;
  proxy: ProxyConfig | null;
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
  created_at: number;
  updated_at: number;
}

export interface RuntimeStatus {
  running: boolean;
  pid: number | null;
  started_at: number | null;
}

export interface ProfileView {
  profile: Profile;
  runtime: RuntimeStatus;
}

export interface ProfileDraft {
  name: string;
  notes: string;
  tags: string[];
  browser: BrowserConfig;
  network: NetworkConfig;
}

export interface AppStatus {
  chromium_ready: boolean;
  chromium_state: string;
  browser_binary: string;
  workspace: string;
}
