import { useState } from "react";
import { ExternalLink, KeyRound } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { api, errorMessage, type StatusPayload } from "../lib/api";
import { Button, Card, ErrorBanner, Field, Input, Logo } from "./ui";

export function SetupScreen({
  onDone,
}: {
  onDone: (status: StatusPayload) => void;
}) {
  const [apiId, setApiId] = useState("");
  const [apiHash, setApiHash] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const submit = async () => {
    setBusy(true);
    setError(null);
    try {
      const status = await api.saveCredentials(Number(apiId), apiHash);
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
              Configura tu API de Telegram
            </h1>
            <p className="mt-0.5 text-sm text-slate-400">
              Usa tus propias credenciales: tu cuenta y tus archivos son solo
              tuyos.
            </p>
          </div>
        </div>

        <ol className="space-y-2.5 rounded-xl border border-white/10 bg-white/[0.03] p-4 text-sm text-slate-300">
          <li>
            <span className="mr-2 font-semibold text-brand-400">1.</span>
            Entra a{" "}
            <button
              type="button"
              className="inline-flex items-center gap-1 text-brand-300 underline-offset-2 hover:underline"
              onClick={() => void openUrl("https://my.telegram.org")}
            >
              my.telegram.org <ExternalLink className="h-3 w-3" />
            </button>{" "}
            e inicia sesión con tu número de teléfono.
          </li>
          <li>
            <span className="mr-2 font-semibold text-brand-400">2.</span>
            Abre «API development tools» y crea una aplicación (cualquier
            nombre sirve).
          </li>
          <li>
            <span className="mr-2 font-semibold text-brand-400">3.</span>
            Copia el <b className="text-slate-100">api_id</b> y el{" "}
            <b className="text-slate-100">api_hash</b> aquí abajo.
          </li>
        </ol>

        <div className="grid grid-cols-1 gap-4 sm:grid-cols-[170px_1fr]">
          <Field label="API ID">
            <Input
              inputMode="numeric"
              placeholder="12345678"
              value={apiId}
              onChange={(event) =>
                setApiId(event.target.value.replace(/\D/g, ""))
              }
            />
          </Field>
          <Field label="API Hash">
            <Input
              placeholder="0123456789abcdef0123456789abcdef"
              value={apiHash}
              onChange={(event) => setApiHash(event.target.value.trim())}
            />
          </Field>
        </div>

        <ErrorBanner message={error} />

        <div className="flex flex-wrap items-center justify-between gap-3">
          <p className="flex items-center gap-1.5 text-xs text-slate-500">
            <KeyRound className="h-3.5 w-3.5" />
            Se guardan solo en tu PC, nunca se envían a terceros. También puedes
            definirlos en un <code className="rounded bg-white/5 px-1">.env</code>{" "}
            (TELEDRIVE_API_ID / TELEDRIVE_API_HASH).
          </p>
          <Button
            loading={busy}
            disabled={!apiId || apiHash.length < 10}
            onClick={() => void submit()}
          >
            Guardar y continuar
          </Button>
        </div>
      </Card>
    </div>
  );
}
