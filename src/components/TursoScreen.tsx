import { useState } from "react";
import { Database, ExternalLink, KeyRound } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage, type StatusPayload } from "../lib/api";
import { Button, Card, ErrorBanner, Field, Input, Logo } from "./ui";

export function TursoScreen({
  onDone,
}: {
  onDone: (status: StatusPayload) => void;
}) {
  const [url, setUrl] = useState("");
  const [token, setToken] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      const status = await api.saveTurso(url, token);
      onDone(status);
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="flex h-full items-center justify-center overflow-y-auto p-8">
      <Card className="w-full max-w-lg animate-fade-up space-y-6">
        <div className="flex items-center gap-4">
          <Logo size={52} />
          <div>
            <h1 className="text-xl font-semibold text-white">
              Conecta tu índice en Turso
            </h1>
            <p className="mt-0.5 text-sm text-slate-400">
              La organización de tu nube (carpetas, nombres, búsqueda) vive en
              Turso. Los archivos viven en tu canal de Telegram.
            </p>
          </div>
        </div>

        <ol className="space-y-2.5 rounded-xl border border-white/10 bg-white/[0.03] p-4 text-sm text-slate-300">
          <li>
            <span className="mr-2 font-semibold text-brand-400">1.</span>
            Crea una cuenta y una base de datos en{" "}
            <button
              type="button"
              className="inline-flex items-center gap-1 text-brand-300 underline-offset-2 hover:underline"
              onClick={() => void openUrl("https://turso.tech")}
            >
              turso.tech <ExternalLink className="h-3 w-3" />
            </button>
          </li>
          <li>
            <span className="mr-2 font-semibold text-brand-400">2.</span>
            Con la CLI:{" "}
            <code className="rounded bg-white/5 px-1.5 py-0.5 text-xs text-brand-200">
              turso db create mi-nube
            </code>
            , luego obtén la URL con{" "}
            <code className="rounded bg-white/5 px-1.5 py-0.5 text-xs text-brand-200">
              turso db show mi-nube --url
            </code>{" "}
            y el token con{" "}
            <code className="rounded bg-white/5 px-1.5 py-0.5 text-xs text-brand-200">
              turso db tokens create mi-nube
            </code>
            .
          </li>
          <li>
            <span className="mr-2 font-semibold text-brand-400">3.</span>
            Pega la URL (empieza con <b>libsql://</b>) y el token aquí abajo.
          </li>
        </ol>

        <div className="space-y-4">
          <Field label="URL de la base de datos">
            <Input
              autoFocus
              placeholder="libsql://mi-nube-tuusuario.turso.io"
              value={url}
              onChange={(event) => setUrl(event.target.value.trim())}
            />
          </Field>
          <Field label="Token de acceso">
            <Input
              type="password"
              placeholder="eyJhbGciOiJIUzI1NiIs..."
              value={token}
              onChange={(event) => setToken(event.target.value.trim())}
              onKeyDown={(event) => {
                if (event.key === "Enter" && url && token) void submit();
              }}
            />
          </Field>
        </div>

        <ErrorBanner message={error} />

        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="flex items-center gap-1.5 text-xs text-slate-500">
            <KeyRound className="h-3.5 w-3.5" />
            El token se guarda solo en tu PC. También puedes definirlo en un{" "}
            <code className="rounded bg-white/5 px-1">.env</code> (
            TELEDRIVE_TURSO_URL / TELEDRIVE_TURSO_TOKEN).
          </p>
          <Button
            loading={busy}
            disabled={!url || token.length < 10}
            onClick={() => void submit()}
          >
            <Database className="h-4 w-4" />
            Conectar Turso
          </Button>
        </div>
      </Card>
    </div>
  );
}
