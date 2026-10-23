import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open, save, confirm } from "@tauri-apps/plugin-dialog";
import {
  AlertCircle,
  Archive,
  CheckCircle2,
  Clock,
  Cloud,
  Download,
  File,
  FileText,
  Film,
  FolderOpen,
  Image as ImageIcon,
  Infinity as InfinityIcon,
  LogOut,
  Music,
  Play,
  RefreshCw,
  RotateCcw,
  Search,
  ShieldCheck,
  Sparkles,
  Star,
  Trash,
  Trash2,
  Upload,
  X,
  Zap,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";
import {
  api,
  errorMessage,
  type DownloadEvent,
  type DriveInfo,
  type FileEntry,
  type Me,
  type Preview,
  type UploadEvent,
} from "../lib/api";
import { Button, Logo, Spinner, cx } from "./ui";

const NAV_ITEMS = [
  { id: "drive", label: "Mi Nube", icon: Cloud },
  { id: "recent", label: "Recientes", icon: Clock },
  { id: "favorites", label: "Favoritos", icon: Star },
  { id: "trash", label: "Papelera", icon: Trash },
] as const;

type SectionId = (typeof NAV_ITEMS)[number]["id"];

type JobKind = "upload" | "download";

type Job = {
  key: string;
  kind: JobKind;
  jobId: string;
  fileId: string;
  name: string;
  phase: string;
  sent: number;
  total: number;
  part: number;
  parts: number;
  message: string | null;
  path?: string;
};

export function MainLayout({
  me,
  onLoggedOut,
}: {
  me: Me;
  onLoggedOut: () => void;
}) {
  const [section, setSection] = useState<SectionId>("drive");
  const [files, setFiles] = useState<FileEntry[]>([]);
  const [loadingFiles, setLoadingFiles] = useState(true);
  const [jobs, setJobs] = useState<Job[]>([]);
  const [dragging, setDragging] = useState(false);
  const [syncing, setSyncing] = useState(false);
  const [drive, setDrive] = useState<DriveInfo | null>(null);
  const [toast, setToast] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [query, setQuery] = useState("");
  const [previews, setPreviews] = useState<Record<string, Preview>>({});
  const toastTimer = useRef<number | null>(null);
  const uploadPathsRef = useRef<(paths: string[]) => void>(() => {});
  const sectionRef = useRef<SectionId>(section);
  sectionRef.current = section;
  const queryRef = useRef(query);
  queryRef.current = query;

  const showToast = useCallback((message: string) => {
    setToast(message);
    if (toastTimer.current) window.clearTimeout(toastTimer.current);
    toastTimer.current = window.setTimeout(() => setToast(null), 4200);
  }, []);

  const refreshFiles = useCallback(async () => {
    try {
      setFiles(await api.listFiles(sectionRef.current, queryRef.current));
    } catch (err) {
      showToast(errorMessage(err));
    } finally {
      setLoadingFiles(false);
    }
  }, [showToast]);

  const runSync = useCallback(async () => {
    setSyncing(true);
    try {
      const report = await api.syncDrive();
      await refreshFiles();
      if (report.imported > 0) {
        showToast(`Sincronizado: ${report.imported} archivo(s) en tu índice`);
      }
    } catch (err) {
      showToast(errorMessage(err));
      setLoadingFiles(false);
    } finally {
      setSyncing(false);
    }
  }, [refreshFiles, showToast]);

  const upsertJob = useCallback(
    (kind: JobKind, payload: UploadEvent | DownloadEvent) => {
      setJobs((prev) => {
        const job: Job = {
          key: payload.jobId,
          kind,
          jobId: payload.jobId,
          fileId: payload.fileId,
          name: payload.name,
          phase: payload.phase,
          sent: payload.sent,
          total: payload.total,
          part: payload.part,
          parts: payload.parts,
          message: payload.message,
        };

        const index = prev.findIndex((item) => item.jobId === payload.jobId);
        if (index >= 0) {
          const next = [...prev];
          next[index] = { ...next[index], ...job };
          return next;
        }
        return [...prev, job].slice(-5);
      });
    },
    [],
  );

  const removeJob = useCallback((jobId: string) => {
    setJobs((prev) => prev.filter((job) => job.jobId !== jobId));
  }, []);

  const uploadPaths = useCallback(
    (paths: string[]) => {
      if (paths.length === 0) return;
      void api
        .uploadFiles(paths)
        .then((summary) => {
          if (summary.skipped > 0 || summary.failed > 0) {
            showToast(
              `Subida: ${summary.done} completos · ${summary.skipped} duplicados · ${summary.failed} errores`,
            );
          }
        })
        .catch((err) => showToast(errorMessage(err)));
    },
    [showToast],
  );

  const downloadFile = useCallback(
    async (file: FileEntry) => {
      try {
        const target = await save({
          title: `Guardar ${file.name}`,
          defaultPath: file.name,
        });
        if (!target) return;

        const result = await api.downloadFile(file.id, target);
        setJobs((prev) =>
          prev.map((job) =>
            job.jobId === result.jobId
              ? {
                  ...job,
                  phase: "done",
                  path: result.path,
                  sent: result.bytes,
                  total: result.bytes,
                }
              : job,
          ),
        );
        showToast(
          result.verified
            ? `Descargado y verificado: ${file.name}`
            : `Descargado: ${file.name}`,
        );
      } catch (err) {
        showToast(errorMessage(err));
      }
    },
    [showToast],
  );

  const toggleFavorite = useCallback(
    async (file: FileEntry) => {
      try {
        const favorite = await api.toggleFavorite(file.id);
        setFiles((prev) =>
          prev.map((item) =>
            item.id === file.id ? { ...item, favorite } : item,
          ),
        );
        if (sectionRef.current === "favorites" && !favorite) {
          void refreshFiles();
        }
      } catch (err) {
        showToast(errorMessage(err));
      }
    },
    [refreshFiles, showToast],
  );

  const moveToTrash = useCallback(
    async (file: FileEntry) => {
      try {
        await api.trashFile(file.id);
        showToast(`«${file.name}» enviado a la papelera`);
        await refreshFiles();
      } catch (err) {
        showToast(errorMessage(err));
      }
    },
    [refreshFiles, showToast],
  );

  const restoreFile = useCallback(
    async (file: FileEntry) => {
      try {
        await api.restoreFile(file.id);
        showToast(`«${file.name}» restaurado`);
        await refreshFiles();
      } catch (err) {
        showToast(errorMessage(err));
      }
    },
    [refreshFiles, showToast],
  );

  const purgeFile = useCallback(
    async (file: FileEntry) => {
      const accepted = await confirm(
        `«${file.name}» se borrará de Telegram y del índice. No se puede deshacer.`,
        {
          title: "Eliminar definitivamente",
          kind: "error",
          okLabel: "Eliminar",
          cancelLabel: "Cancelar",
        },
      );
      if (!accepted) return;

      try {
        await api.purgeFile(file.id);
        showToast(`«${file.name}» eliminado definitivamente`);
        await refreshFiles();
      } catch (err) {
        showToast(errorMessage(err));
      }
    },
    [refreshFiles, showToast],
  );

  useEffect(() => {
    const timer = window.setTimeout(() => setQuery(search.trim()), 250);
    return () => window.clearTimeout(timer);
  }, [search]);

  useEffect(() => {
    void refreshFiles();
  }, [refreshFiles, section, query]);

  useEffect(() => {
    const pending = files
      .filter((file) => isPreviewable(file.name) && !previews[file.id])
      .slice(0, 30);
    if (pending.length === 0) return;

    let cancelled = false;
    void Promise.all(
      pending.map(async (file) => {
        try {
          const preview = await api.filePreview(file.id);
          if (cancelled || !preview) return;
          setPreviews((prev) => ({ ...prev, [file.id]: preview }));
        } catch {
          return;
        }
      }),
    );

    return () => {
      cancelled = true;
    };
  }, [files, previews]);

  uploadPathsRef.current = uploadPaths;

  useEffect(() => {
    let unlistenUpload: (() => void) | undefined;
    let unlistenDownload: (() => void) | undefined;

    void listen<UploadEvent>("upload://progress", (event) => {
      const payload = event.payload;
      upsertJob("upload", payload);

      if (payload.phase === "done" || payload.phase === "skipped") {
        void refreshFiles();
      }
      if (
        payload.phase === "done" ||
        payload.phase === "skipped" ||
        payload.phase === "error"
      ) {
        window.setTimeout(() => {
          setJobs((prev) => prev.filter((job) => job.jobId !== payload.jobId));
        }, 6000);
      }
    }).then((fn) => {
      unlistenUpload = fn;
    });

    void listen<DownloadEvent>("download://progress", (event) => {
      const payload = event.payload;
      upsertJob("download", payload);
    }).then((fn) => {
      unlistenDownload = fn;
    });

    void runSync();
    void api
      .driveInfo()
      .then(setDrive)
      .catch(() => {});

    return () => {
      unlistenUpload?.();
      unlistenDownload?.();
    };
  }, [refreshFiles, runSync, upsertJob]);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          setDragging(true);
        } else if (payload.type === "leave") {
          setDragging(false);
        } else if (payload.type === "drop") {
          setDragging(false);
          uploadPathsRef.current(payload.paths);
        }
      })
      .then((fn) => {
        unlisten = fn;
      });
    return () => unlisten?.();
  }, []);

  const pickFiles = async () => {
    const selected = await open({
      multiple: true,
      directory: false,
      title: "Selecciona archivos para tu nube",
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    uploadPaths(paths);
  };

  const displayName =
    [me.firstName, me.lastName].filter(Boolean).join(" ") || "Mi cuenta";
  const initials = displayName
    .split(" ")
    .map((part) => part.charAt(0))
    .slice(0, 2)
    .join("")
    .toUpperCase();

  const activeLabel =
    NAV_ITEMS.find((item) => item.id === section)?.label ?? "Mi Nube";

  return (
    <div className="flex h-full">
      <aside className="flex w-64 shrink-0 flex-col border-r border-white/5 bg-ink-900/60 backdrop-blur-xl">
        <div className="flex items-center gap-3 px-5 pt-6 pb-5">
          <Logo size={38} />
          <div>
            <p className="text-sm font-semibold text-white">TeleDrive</p>
            <p className="text-[11px] text-slate-500">Nube ilimitada</p>
          </div>
        </div>

        <nav className="flex-1 space-y-1 px-3">
          {NAV_ITEMS.map((item) => {
            const Icon = item.icon;
            const active = section === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => setSection(item.id)}
                className={cx(
                  "group flex w-full items-center gap-3 rounded-xl px-3 py-2.5 text-sm transition-all duration-200",
                  active
                    ? "bg-white/[0.07] text-white shadow-sm"
                    : "text-slate-400 hover:translate-x-0.5 hover:bg-white/[0.04] hover:text-slate-200",
                )}
              >
                <Icon
                  className={cx(
                    "h-4 w-4 transition-colors",
                    active
                      ? "text-brand-400"
                      : "text-slate-500 group-hover:text-slate-300",
                  )}
                />
                {item.label}
                {active && (
                  <span className="ml-auto h-1.5 w-1.5 rounded-full bg-brand-400" />
                )}
              </button>
            );
          })}
        </nav>

        <div className="border-t border-white/5 p-4">
          <div className="flex items-center gap-3 rounded-xl bg-white/[0.03] p-3">
            <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-brand-500 to-violet-500 text-sm font-semibold text-white">
              {initials || "T"}
            </div>
            <div className="min-w-0 flex-1">
              <p className="truncate text-sm font-medium text-white">
                {displayName}
              </p>
              <p className="truncate text-xs text-slate-500">
                {me.username ? `@${me.username}` : (me.phone ?? "Telegram")}
              </p>
            </div>
            <button
              type="button"
              title="Cerrar sesión"
              className="rounded-lg p-2 text-slate-400 transition-colors hover:bg-white/5 hover:text-rose-300"
              onClick={() => void api.logout().then(() => onLoggedOut())}
            >
              <LogOut className="h-4 w-4" />
            </button>
          </div>
          <p className="mt-3 flex items-center gap-1.5 px-1 text-[11px] text-slate-500">
            <InfinityIcon className="h-3.5 w-3.5 text-brand-400" />
            Ilimitado
            {drive && (
              <>
                {" · "}
                {formatBytes(drive.partLimitBytes)}/archivo
                {drive.premium ? " (Premium)" : ""}
              </>
            )}
          </p>
        </div>
      </aside>

      <main className="relative flex min-w-0 flex-1 flex-col">
        <header className="flex items-center gap-4 border-b border-white/5 px-6 py-4">
          <div>
            <h1 className="text-base font-semibold text-white">{activeLabel}</h1>
            <p className="text-xs text-slate-500">
              {drive ? `Canal «${drive.title}» en tu cuenta de Telegram` : "Tu canal privado respaldado por Telegram"}
            </p>
          </div>
          <div className="ml-auto flex items-center gap-2">
            <div className="group relative hidden md:block">
              <Search className="pointer-events-none absolute top-1/2 left-3 h-4 w-4 -translate-y-1/2 text-slate-500" />
              <input
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                placeholder="Buscar en tu nube…"
                className="w-56 rounded-xl border border-white/10 bg-white/[0.04] py-2 pr-8 pl-9 text-sm text-slate-200 placeholder:text-slate-600 transition-all focus:w-72 focus:border-brand-400/60 focus:bg-white/[0.06] focus:ring-2 focus:ring-brand-500/20"
              />
              {search && (
                <button
                  type="button"
                  title="Limpiar búsqueda"
                  onClick={() => setSearch("")}
                  className="absolute top-1/2 right-2 -translate-y-1/2 rounded-md p-1 text-slate-500 transition-colors hover:bg-white/10 hover:text-slate-300"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              )}
            </div>
            <Button
              variant="ghost"
              disabled={syncing}
              onClick={() => void runSync()}
              title="Reconstruir el índice desde Telegram"
            >
              <RefreshCw className={cx("h-4 w-4", syncing && "animate-spin")} />
              {syncing ? "Sincronizando…" : "Sincronizar"}
            </Button>
            <Button onClick={() => void pickFiles()}>
              <Upload className="h-4 w-4" />
              Subir
            </Button>
          </div>
        </header>

        <div className="flex-1 overflow-y-auto">
          {loadingFiles ? (
            <div className="flex h-full items-center justify-center gap-2 text-sm text-slate-400">
              <Spinner /> Cargando tu nube…
            </div>
          ) : files.length === 0 ? (
            section === "drive" ? (
              <EmptyState dragging={dragging} onPick={() => void pickFiles()} />
            ) : (
              <SectionEmpty section={section} query={query} />
            )
          ) : (
            <div className="p-6">
              <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3">
                {files.map((file) => (
                  <FileCard
                    key={file.id}
                    file={file}
                    section={section}
                    preview={previews[file.id]}
                    onDownload={() => void downloadFile(file)}
                    onToggleFavorite={() => void toggleFavorite(file)}
                    onTrash={() => void moveToTrash(file)}
                    onRestore={() => void restoreFile(file)}
                    onPurge={() => void purgeFile(file)}
                  />
                ))}
              </div>
            </div>
          )}
        </div>

        {dragging && (
          <div className="pointer-events-none absolute inset-0 z-40 flex items-center justify-center bg-ink-950/60 backdrop-blur-sm">
            <div className="flex items-center gap-3 rounded-2xl border border-brand-400/40 bg-ink-800/90 px-6 py-4 shadow-2xl">
              <Sparkles className="h-5 w-5 text-brand-300" />
              <span className="text-sm font-medium text-white">
                Suelta para subir a tu nube
              </span>
            </div>
          </div>
        )}

        {jobs.length > 0 && (
          <div className="fixed right-6 bottom-6 z-50 w-80 space-y-2">
            {jobs.map((job) => (
              <JobCard
                key={job.key}
                job={job}
                onClose={() => removeJob(job.jobId)}
              />
            ))}
          </div>
        )}

        {toast && (
          <div className="fixed top-6 left-1/2 z-[60] max-w-sm -translate-x-1/2 animate-fade-up rounded-xl border border-white/10 bg-ink-800/95 px-4 py-3 text-sm text-slate-100 shadow-2xl backdrop-blur">
            {toast}
          </div>
        )}
      </main>
    </div>
  );
}

function EmptyState({
  dragging,
  onPick,
}: {
  dragging: boolean;
  onPick: () => void;
}) {
  return (
    <div className="flex min-h-full items-center justify-center p-8">
      <div className="w-full max-w-2xl animate-fade-up space-y-6 text-center">
        <div
          className={cx(
            "rounded-3xl border-2 border-dashed p-12 transition-all duration-300",
            dragging
              ? "scale-[1.015] border-brand-400/70 bg-brand-500/10"
              : "border-white/10 bg-white/[0.02]",
          )}
        >
          <div className="mx-auto mb-5 flex h-16 w-16 items-center justify-center rounded-2xl bg-gradient-to-br from-brand-500/25 to-violet-500/25">
            <Upload className="h-7 w-7 text-brand-300" />
          </div>
          <h2 className="text-xl font-semibold text-white">
            Tu nube está lista y vacía
          </h2>
          <p className="mx-auto mt-2 max-w-md text-sm text-slate-400">
            Arrastra y suelta archivos aquí, o usa el botón Subir. Cada archivo
            se guarda en tu canal privado de Telegram y se indexa en Turso.
          </p>
          <div className="mt-6">
            <Button onClick={onPick}>
              <Upload className="h-4 w-4" />
              Subir mis primeros archivos
            </Button>
          </div>
        </div>

        <div className="grid grid-cols-1 gap-3 sm:grid-cols-3">
          <FeatureCard
            icon={InfinityIcon}
            title="Ilimitado"
            text="Un canal privado sin cuota de almacenamiento."
          />
          <FeatureCard
            icon={Zap}
            title="Archivos gigantes"
            text="Troceo automático: un video de 20 GB es un solo archivo."
          />
          <FeatureCard
            icon={ShieldCheck}
            title="Tu cuenta, tus datos"
            text="Sin servidores intermedios ni cuentas de terceros."
          />
        </div>
      </div>
    </div>
  );
}

function SectionEmpty({
  section,
  query,
}: {
  section: SectionId;
  query: string;
}) {
  const copy = {
    drive: {
      icon: Cloud,
      title: "Tu nube está lista y vacía",
      text: "Sube un archivo para empezar.",
    },
    recent: {
      icon: Clock,
      title: "Nada por aquí",
      text: "Los archivos subidos en los últimos 7 días aparecen en esta vista.",
    },
    favorites: {
      icon: Star,
      title: "Sin favoritos",
      text: "Marca un archivo con la estrella para encontrarlo rápido.",
    },
    trash: {
      icon: Trash,
      title: "Papelera vacía",
      text: "Los archivos que muevas aquí se pueden restaurar o borrar para siempre.",
    },
  }[section];
  const Icon = copy.icon;

  if (query) {
    return (
      <div className="flex min-h-full items-center justify-center p-8">
        <div className="animate-fade-up rounded-2xl border border-white/10 bg-white/[0.02] px-10 py-12 text-center">
          <Search className="mx-auto mb-3 h-8 w-8 text-slate-500" />
          <p className="text-sm font-medium text-slate-200">
            Sin resultados para «{query}»
          </p>
          <p className="mt-1 max-w-xs text-xs text-slate-500">
            Prueba con otro nombre o limpia la búsqueda.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="flex min-h-full items-center justify-center p-8">
      <div className="animate-fade-up rounded-2xl border border-white/10 bg-white/[0.02] px-10 py-12 text-center">
        <Icon className="mx-auto mb-3 h-8 w-8 text-slate-500" />
        <p className="text-sm font-medium text-slate-200">{copy.title}</p>
        <p className="mt-1 max-w-xs text-xs text-slate-500">{copy.text}</p>
      </div>
    </div>
  );
}

function FileCard({
  file,
  section,
  preview,
  onDownload,
  onToggleFavorite,
  onTrash,
  onRestore,
  onPurge,
}: {
  file: FileEntry;
  section: SectionId;
  preview?: Preview;
  onDownload: () => void;
  onToggleFavorite: () => void;
  onTrash: () => void;
  onRestore: () => void;
  onPurge: () => void;
}) {
  const Icon = fileIcon(file.name);

  const body = (
    <div className="flex items-start gap-3 p-4">
      {!preview && (
        <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl bg-gradient-to-br from-brand-500/20 to-violet-500/20">
          <Icon className="h-5 w-5 text-brand-300" />
        </div>
      )}
      <div className="min-w-0 flex-1">
        <p className="truncate text-sm font-medium text-white" title={file.name}>
          {file.name}
        </p>
        <p className="mt-0.5 text-xs text-slate-500">
          {formatBytes(file.size)} · {formatDate(file.createdAt)}
        </p>
        <div className="mt-2 flex flex-wrap items-center gap-1.5">
          {file.parts > 1 && (
            <span className="rounded-full bg-white/5 px-2 py-0.5 text-[10px] font-medium text-slate-400">
              {file.parts} partes
            </span>
          )}
          {file.status === "partial" && (
            <span className="rounded-full bg-amber-500/15 px-2 py-0.5 text-[10px] font-medium text-amber-300">
              Parcial
            </span>
          )}
          {file.status === "error" && (
            <span className="rounded-full bg-rose-500/15 px-2 py-0.5 text-[10px] font-medium text-rose-300">
              Error
            </span>
          )}
          {file.status === "uploading" && (
            <span className="rounded-full bg-brand-500/15 px-2 py-0.5 text-[10px] font-medium text-brand-300">
              Subiendo…
            </span>
          )}
        </div>
      </div>
      <FileActions
        file={file}
        section={section}
        onDownload={onDownload}
        onToggleFavorite={onToggleFavorite}
        onTrash={onTrash}
        onRestore={onRestore}
        onPurge={onPurge}
      />
    </div>
  );

  if (!preview) {
    return (
      <div className="group rounded-2xl border border-white/10 bg-white/[0.03] transition-all duration-200 hover:-translate-y-0.5 hover:border-brand-400/30 hover:bg-white/[0.05]">
        {body}
      </div>
    );
  }

  return (
    <div className="group overflow-hidden rounded-2xl border border-white/10 bg-white/[0.03] transition-all duration-200 hover:-translate-y-0.5 hover:border-brand-400/30 hover:bg-white/[0.05]">
      <div className="relative h-32 w-full overflow-hidden bg-ink-900">
        <img
          src={preview.dataUrl}
          alt={file.name}
          className="h-full w-full object-cover"
          loading="lazy"
        />
        {preview.kind === "video" && (
          <span className="absolute inset-0 flex items-center justify-center bg-black/25">
            <span className="flex h-9 w-9 items-center justify-center rounded-full bg-black/55 backdrop-blur">
              <Play className="h-4 w-4 text-white" />
            </span>
          </span>
        )}
        {preview.duration ? (
          <span className="absolute right-2 bottom-2 rounded-md bg-black/65 px-1.5 py-0.5 text-[10px] font-medium text-white">
            {formatDuration(preview.duration)}
          </span>
        ) : null}
      </div>
      {body}
    </div>
  );
}

function FileActions({
  file,
  section,
  onDownload,
  onToggleFavorite,
  onTrash,
  onRestore,
  onPurge,
}: {
  file: FileEntry;
  section: SectionId;
  onDownload: () => void;
  onToggleFavorite: () => void;
  onTrash: () => void;
  onRestore: () => void;
  onPurge: () => void;
}) {
  const downloadable = file.status === "complete";
  const inTrash = section === "trash";

  return (
    <div className="flex shrink-0 flex-col gap-1">
      <button
        type="button"
        onClick={onToggleFavorite}
        title={file.favorite ? "Quitar de favoritos" : "Marcar como favorito"}
        className="rounded-lg p-1.5 text-slate-500 transition-colors hover:bg-white/10 hover:text-amber-300"
      >
        <Star
          className={cx(
            "h-4 w-4",
            file.favorite && "fill-amber-400 text-amber-400",
          )}
        />
      </button>
      <button
        type="button"
        onClick={onDownload}
        disabled={!downloadable}
        title={
          downloadable ? "Descargar a mi equipo" : "El archivo está incompleto"
        }
        className="rounded-lg p-1.5 text-slate-400 transition-colors hover:bg-white/10 hover:text-white disabled:cursor-not-allowed disabled:text-slate-700 disabled:hover:bg-transparent"
      >
        <Download className="h-4 w-4" />
      </button>
      {inTrash ? (
        <>
          <button
            type="button"
            onClick={onRestore}
            title="Restaurar"
            className="rounded-lg p-1.5 text-slate-400 transition-colors hover:bg-white/10 hover:text-emerald-300"
          >
            <RotateCcw className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={onPurge}
            title="Eliminar definitivamente"
            className="rounded-lg p-1.5 text-slate-400 transition-colors hover:bg-rose-500/15 hover:text-rose-300"
          >
            <Trash2 className="h-4 w-4" />
          </button>
        </>
      ) : (
        <button
          type="button"
          onClick={onTrash}
          title="Mover a la papelera"
          className="rounded-lg p-1.5 text-slate-400 transition-colors hover:bg-white/10 hover:text-rose-300"
        >
          <Trash2 className="h-4 w-4" />
        </button>
      )}
    </div>
  );
}

function JobCard({ job, onClose }: { job: Job; onClose: () => void }) {
  const progress =
    job.total > 0 ? Math.min(100, Math.round((job.sent / job.total) * 100)) : 0;

  const label =
    job.kind === "upload"
      ? job.phase === "hashing"
        ? "Calculando huella…"
        : job.phase === "uploading"
          ? job.parts > 1
            ? `Subiendo parte ${job.part}/${job.parts} · ${progress}%`
            : `Subiendo · ${progress}%`
          : job.phase === "done"
            ? "Completado"
            : job.phase === "skipped"
              ? "Duplicado (omitido)"
              : (job.message ?? "Error")
      : job.phase === "downloading"
        ? job.parts > 1
          ? `Descargando parte ${job.part}/${job.parts} · ${progress}%`
          : `Descargando · ${progress}%`
        : job.phase === "verifying"
          ? "Verificando huella…"
          : job.phase === "done"
            ? "Descargado y verificado"
            : (job.message ?? "Error");

  const finished =
    job.phase === "error" ||
    job.phase === "done" ||
    job.phase === "skipped" ||
    job.phase === "verifying";

  return (
    <div className="animate-fade-up rounded-xl border border-white/10 bg-ink-800/95 p-3 shadow-2xl backdrop-blur">
      <div className="flex items-center gap-2.5">
        {job.phase === "done" || job.phase === "skipped" ? (
          <CheckCircle2 className="h-4 w-4 shrink-0 text-emerald-400" />
        ) : job.phase === "error" ? (
          <AlertCircle className="h-4 w-4 shrink-0 text-rose-400" />
        ) : (
          <Spinner className="shrink-0 text-brand-300" />
        )}
        <p className="truncate text-xs font-medium text-slate-100" title={job.name}>
          {job.name}
        </p>
        <button
          type="button"
          onClick={onClose}
          title="Cerrar"
          className="ml-auto rounded-md p-1 text-slate-500 transition-colors hover:bg-white/5 hover:text-slate-300"
        >
          <X className="h-3.5 w-3.5" />
        </button>
      </div>
      <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-white/10">
        <div
          className={cx(
            "h-full rounded-full transition-all duration-300",
            job.phase === "error"
              ? "bg-rose-500"
              : job.phase === "done" || job.phase === "skipped"
                ? "bg-emerald-500"
                : "bg-gradient-to-r from-brand-500 to-violet-500",
          )}
          style={{ width: `${finished ? 100 : progress}%` }}
        />
      </div>
      <p className="mt-1.5 truncate text-[11px] text-slate-400">{label}</p>
      {job.kind === "download" && job.phase === "done" && job.path && (
        <button
          type="button"
          onClick={() => void api.revealPath(job.path as string)}
          className="mt-2 inline-flex items-center gap-1.5 rounded-lg bg-white/5 px-2 py-1 text-[11px] text-slate-300 transition-colors hover:bg-white/10 hover:text-white"
        >
          <FolderOpen className="h-3.5 w-3.5" />
          Abrir carpeta
        </button>
      )}
    </div>
  );
}

function FeatureCard({
  icon: Icon,
  title,
  text,
}: {
  icon: LucideIcon;
  title: string;
  text: string;
}) {
  return (
    <div className="rounded-2xl border border-white/10 bg-white/[0.03] p-4 text-left transition-colors hover:bg-white/[0.05]">
      <Icon className="mb-2.5 h-5 w-5 text-brand-300" />
      <p className="text-sm font-medium text-white">{title}</p>
      <p className="mt-1 text-xs leading-relaxed text-slate-500">{text}</p>
    </div>
  );
}

function fileIcon(name: string): LucideIcon {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  if (["png", "jpg", "jpeg", "gif", "webp", "bmp", "svg", "avif"].includes(ext))
    return ImageIcon;
  if (["mp4", "mkv", "mov", "avi", "webm", "wmv"].includes(ext)) return Film;
  if (["mp3", "wav", "flac", "ogg", "m4a", "aac"].includes(ext)) return Music;
  if (["zip", "rar", "7z", "tar", "gz"].includes(ext)) return Archive;
  if (
    ["pdf", "doc", "docx", "txt", "md", "xls", "xlsx", "ppt", "pptx"].includes(
      ext,
    )
  )
    return FileText;
  return File;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes;
  let index = -1;
  do {
    value /= 1024;
    index += 1;
  } while (value >= 1024 && index < units.length - 1);
  return `${value.toFixed(value >= 10 ? 0 : 1)} ${units[index]}`;
}

function formatDate(seconds: number): string {
  if (!seconds) return "—";
  return new Date(seconds * 1000).toLocaleDateString("es", {
    day: "2-digit",
    month: "short",
    year: "numeric",
  });
}

function isPreviewable(name: string): boolean {
  const ext = name.split(".").pop()?.toLowerCase() ?? "";
  return [
    "png",
    "jpg",
    "jpeg",
    "gif",
    "webp",
    "bmp",
    "avif",
    "heic",
    "mp4",
    "mkv",
    "mov",
    "avi",
    "webm",
    "wmv",
    "m4v",
  ].includes(ext);
}

function formatDuration(seconds: number): string {
  const total = Math.round(seconds);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const rest = total % 60;
  const pad = (value: number) => value.toString().padStart(2, "0");
  return hours > 0
    ? `${hours}:${pad(minutes)}:${pad(rest)}`
    : `${minutes}:${pad(rest)}`;
}
