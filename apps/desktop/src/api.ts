import { invoke } from "@tauri-apps/api/core";
import type {
  AppStatus,
  DiagnosticItem,
  FingerprintHistoryEntry,
  FingerprintSnapshot,
  NetworkProbe,
  ProfileDraft,
  ProfileView,
} from "./types";

type TauriWindow = Window & { __TAURI_INTERNALS__?: unknown };

function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (!(window as TauriWindow).__TAURI_INTERNALS__) {
    return Promise.reject(
      new Error(
        "Dravyn backend is unavailable. Open the native Tauri window with `pnpm tauri dev` or `dravyn desktop`, not the Vite URL in a normal browser.",
      ),
    );
  }
  return invoke<T>(command, args);
}

export const api = {
  appStatus: () => call<AppStatus>("app_status"),
  listProfiles: () => call<ProfileView[]>("list_profiles"),
  createProfile: (draft: ProfileDraft) => call<ProfileView>("create_profile", { draft }),
  updateProfile: (id: string, draft: ProfileDraft) =>
    call<ProfileView>("update_profile", { id, draft }),
  launchProfile: (id: string) => call<ProfileView>("launch_profile", { id }),
  stopProfile: (id: string) => call<ProfileView>("stop_profile", { id }),
  resetProfile: (id: string) => call<ProfileView>("reset_profile", { id }),
  deleteProfile: (id: string) => call<void>("delete_profile", { id }),
  fingerprintHistory: (id: string) =>
    call<FingerprintHistoryEntry[]>("fingerprint_history", { id }),
  fingerprintLatest: (id: string) =>
    call<FingerprintSnapshot | null>("fingerprint_latest", { id }),
  setFingerprintBaseline: (id: string) =>
    call<ProfileView>("set_fingerprint_baseline", { id }),
  networkProbe: (id: string) => call<NetworkProbe>("network_probe", { id }),
  systemDiagnostics: () => call<DiagnosticItem[]>("system_diagnostics"),
  openPrivacyAudit: (id: string) => call<ProfileView>("open_privacy_audit", { id }),
};
