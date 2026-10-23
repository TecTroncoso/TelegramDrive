import { useState } from "react";
import { ArrowLeft, Lock, Phone, ShieldCheck } from "lucide-react";
import { api, errorMessage, type Me } from "../lib/api";
import { Button, Card, ErrorBanner, Field, Input, Logo } from "./ui";

type Step = "phone" | "code" | "password";

export function LoginScreen({
  onLoggedIn,
  onBackToSetup,
}: {
  onLoggedIn: (me: Me) => void;
  onBackToSetup: () => void;
}) {
  const [step, setStep] = useState<Step>("phone");
  const [phone, setPhone] = useState("+");
  const [code, setCode] = useState("");
  const [password, setPassword] = useState("");
  const [hint, setHint] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const run = async (task: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try {
      await task();
    } catch (err) {
      setError(errorMessage(err));
    } finally {
      setBusy(false);
    }
  };

  const sendCode = () =>
    void run(async () => {
      await api.requestCode(phone);
      setStep("code");
    });

  const submitCode = () =>
    void run(async () => {
      const outcome = await api.submitCode(code);
      if (outcome.status === "ok" && outcome.me) {
        onLoggedIn(outcome.me);
      } else {
        setHint(outcome.hint ?? null);
        setStep("password");
      }
    });

  const submitPassword = () =>
    void run(async () => {
      const me = await api.submitPassword(password);
      onLoggedIn(me);
    });

  const titles: Record<Step, { title: string; subtitle: string }> = {
    phone: {
      title: "Inicia sesión en Telegram",
      subtitle: "Usaremos tu propia cuenta para crear tu nube privada.",
    },
    code: {
      title: "Revisa tu Telegram",
      subtitle: `Enviamos un código de acceso al ${phone}.`,
    },
    password: {
      title: "Verificación en dos pasos",
      subtitle: hint
        ? `Tu cuenta pide una contraseña. Pista: ${hint}`
        : "Tu cuenta pide tu contraseña de Telegram.",
    },
  };

  return (
    <div className="flex h-full items-center justify-center overflow-y-auto p-8">
      <Card className="w-full max-w-md animate-fade-up space-y-6">
        <div className="flex items-start gap-4">
          {step !== "phone" ? (
            <button
              type="button"
              className="mt-1 rounded-lg p-1.5 text-slate-400 transition-colors hover:bg-white/5 hover:text-white"
              onClick={() => {
                setStep("phone");
                setCode("");
                setPassword("");
                setError(null);
              }}
            >
              <ArrowLeft className="h-4 w-4" />
            </button>
          ) : (
            <Logo size={46} />
          )}
          <div>
            <h1 className="text-xl font-semibold text-white">
              {titles[step].title}
            </h1>
            <p className="mt-0.5 text-sm text-slate-400">
              {titles[step].subtitle}
            </p>
          </div>
        </div>

        {step === "phone" && (
          <div className="animate-fade-in space-y-4">
            <Field label="Número de teléfono" hint="Incluye el código de país, ej. +503 7000 0000">
              <Input
                autoFocus
                placeholder="+503 7000 0000"
                value={phone}
                onChange={(event) => setPhone(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && phone.length >= 8) void sendCode();
                }}
              />
            </Field>
            <Button
              className="w-full"
              loading={busy}
              disabled={phone.replace(/\D/g, "").length < 7}
              onClick={sendCode}
            >
              <Phone className="h-4 w-4" />
              Enviar código
            </Button>
          </div>
        )}

        {step === "code" && (
          <div className="animate-fade-in space-y-4">
            <Field label="Código de acceso" hint="Llega a tu app de Telegram o por SMS.">
              <Input
                autoFocus
                inputMode="numeric"
                placeholder="12345"
                className="text-center text-lg tracking-[0.4em]"
                value={code}
                onChange={(event) =>
                  setCode(event.target.value.replace(/\D/g, "").slice(0, 6))
                }
                onKeyDown={(event) => {
                  if (event.key === "Enter" && code.length >= 4) void submitCode();
                }}
              />
            </Field>
            <Button
              className="w-full"
              loading={busy}
              disabled={code.length < 4}
              onClick={submitCode}
            >
              <ShieldCheck className="h-4 w-4" />
              Iniciar sesión
            </Button>
            <button
              type="button"
              className="w-full text-center text-xs text-slate-500 transition-colors hover:text-slate-300"
              disabled={busy}
              onClick={sendCode}
            >
              ¿No llegó? Reenviar código
            </button>
          </div>
        )}

        {step === "password" && (
          <div className="animate-fade-in space-y-4">
            <Field label="Contraseña de Telegram">
              <Input
                autoFocus
                type="password"
                placeholder="••••••••"
                value={password}
                onChange={(event) => setPassword(event.target.value)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && password.length > 0)
                    void submitPassword();
                }}
              />
            </Field>
            <Button
              className="w-full"
              loading={busy}
              disabled={password.length === 0}
              onClick={submitPassword}
            >
              <Lock className="h-4 w-4" />
              Desbloquear mi nube
            </Button>
          </div>
        )}

        <ErrorBanner message={error} />

        <button
          type="button"
          className="w-full text-center text-xs text-slate-600 transition-colors hover:text-slate-400"
          onClick={onBackToSetup}
        >
          Usar otras credenciales de API
        </button>
      </Card>
    </div>
  );
}
