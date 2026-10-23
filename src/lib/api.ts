import { invoke } from "@tauri-apps/api/core";

export interface Me {
  id: number;
  firstName: string | null;
  lastName: string | null;
  username: string | null;
  phone: string | null;
}

export interface StatusPayload {
  configured: boolean;
  authorized: boolean;
  me: Me | null;
  tursoConfigured: boolean;
}

export interface LoginOutcome {
  status: "ok" | "passwordRequired";
  me: Me | null;
  hint: string | null;
}

export interface FileEntry {
  id: string;
  name: string;
  folder: string;
  size: number;
  status: string;
  parts: number;
  createdAt: number;
  favorite: boolean;
  trashed: boolean;
}

export interface UploadEvent {
  jobId: string;
  fileId: string;
  name: string;
  phase: "hashing" | "uploading" | "done" | "skipped" | "error";
  sent: number;
  total: number;
  part: number;
  parts: number;
  message: string | null;
}

export interface UploadSummary {
  done: number;
  skipped: number;
  failed: number;
}

export interface DownloadEvent {
  jobId: string;
  fileId: string;
  name: string;
  phase: "downloading" | "verifying" | "done" | "error";
  sent: number;
  total: number;
  part: number;
  parts: number;
  message: string | null;
}

export interface DownloadResult {
  jobId: string;
  path: string;
  bytes: number;
  sha256: string;
  verified: boolean;
}

export interface SyncReport {
  scanned: number;
  imported: number;
}

export interface DriveInfo {
  chatId: number;
  title: string;
  premium: boolean;
  partLimitBytes: number;
}

export interface Preview {
  dataUrl: string;
  kind: "image" | "video";
  duration: number | null;
  resolution: [number, number] | null;
}

export const api = {
  status: () => invoke<StatusPayload>("app_status"),
  saveCredentials: (apiId: number, apiHash: string) =>
    invoke<StatusPayload>("save_credentials", { apiId, apiHash }),
  requestCode: (phone: string) => invoke<void>("request_code", { phone }),
  submitCode: (code: string) => invoke<LoginOutcome>("submit_code", { code }),
  submitPassword: (password: string) =>
    invoke<Me>("submit_password", { password }),
  logout: () => invoke<StatusPayload>("logout"),
  saveTurso: (url: string, token: string) =>
    invoke<StatusPayload>("save_turso", { url, token }),
  listFiles: (view?: string | null, search?: string | null) =>
    invoke<FileEntry[]>("list_files", {
      view: view ?? "drive",
      search: search ?? null,
    }),
  toggleFavorite: (fileId: string) =>
    invoke<boolean>("toggle_favorite", { fileId }),
  trashFile: (fileId: string) => invoke<void>("trash_file", { fileId }),
  restoreFile: (fileId: string) => invoke<void>("restore_file", { fileId }),
  purgeFile: (fileId: string) => invoke<void>("purge_file", { fileId }),
  uploadFiles: (paths: string[]) =>
    invoke<UploadSummary>("upload_files", { paths }),
  syncDrive: () => invoke<SyncReport>("sync_drive"),
  driveInfo: () => invoke<DriveInfo>("drive_info"),
  downloadFile: (fileId: string, path: string) =>
    invoke<DownloadResult>("download_file", { fileId, path }),
  revealPath: (path: string) => invoke<void>("reveal_path", { path }),
  filePreview: (fileId: string) => invoke<Preview | null>("file_preview", { fileId }),
};

export function errorMessage(err: unknown): string {
  if (typeof err === "string") return err;
  if (err instanceof Error) return err.message;
  return String(err);
}
