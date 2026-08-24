import { invoke } from "@tauri-apps/api/core";
import type { AppStatus, ProfileDraft, ProfileView } from "./types";

export const api = {
  appStatus: () => invoke<AppStatus>("app_status"),
  listProfiles: () => invoke<ProfileView[]>("list_profiles"),
  createProfile: (draft: ProfileDraft) => invoke<ProfileView>("create_profile", { draft }),
  updateProfile: (id: string, draft: ProfileDraft) =>
    invoke<ProfileView>("update_profile", { id, draft }),
  launchProfile: (id: string) => invoke<ProfileView>("launch_profile", { id }),
  stopProfile: (id: string) => invoke<ProfileView>("stop_profile", { id }),
  resetProfile: (id: string) => invoke<ProfileView>("reset_profile", { id }),
  deleteProfile: (id: string) => invoke<void>("delete_profile", { id }),
};
