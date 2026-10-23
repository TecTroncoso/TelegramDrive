import { useCallback, useEffect, useState, type ReactNode } from "react";
import { api, errorMessage, type Me } from "./lib/api";
import { LoginScreen } from "./components/LoginScreen";
import { MainLayout } from "./components/MainLayout";
import { SetupScreen } from "./components/SetupScreen";
import { TursoScreen } from "./components/TursoScreen";
import { Button, Card, ErrorBanner, Logo, Spinner } from "./components/ui";

type Phase = "loading" | "error" | "setup" | "login" | "turso" | "ready";

export default function App() {
  const [phase, setPhase] = useState<Phase>("loading");
  const [me, setMe] = useState<Me | null>(null);
  const [bootError, setBootError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setPhase("loading");
    try {
      const status = await api.status();
      if (!status.configured) {
        setPhase("setup");
      } else if (!status.authorized) {
        setPhase("login");
      } else if (!status.tursoConfigured) {
        setMe(status.me);
        setPhase("turso");
      } else {
        setMe(status.me);
        setPhase("ready");
      }
    } catch (err) {
      setBootError(errorMessage(err));
      setPhase("error");
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  return (
    <div className="relative h-full overflow-hidden">
      <Ambient />
      <div className="relative z-10 h-full">
        {phase === "loading" && <BootSplash />}

        {phase === "error" && (
          <CenteredCard title="No se pudo conectar con Telegram">
            <ErrorBanner message={bootError} />
            <div className="flex flex-wrap gap-3 pt-2">
              <Button onClick={() => void refresh()}>Reintentar</Button>
              <Button variant="ghost" onClick={() => setPhase("setup")}>
                Cambiar credenciales
              </Button>
            </div>
          </CenteredCard>
        )}

        {phase === "setup" && (
          <SetupScreen
            onDone={(status) => {
              if (status.authorized && status.tursoConfigured) {
                setMe(status.me);
                setPhase("ready");
              } else if (status.authorized) {
                setMe(status.me);
                setPhase("turso");
              } else {
                setPhase("login");
              }
            }}
          />
        )}

        {phase === "login" && (
          <LoginScreen
            onLoggedIn={() => void refresh()}
            onBackToSetup={() => setPhase("setup")}
          />
        )}

        {phase === "turso" && (
          <TursoScreen
            onDone={(status) => {
              if (status.me) setMe(status.me);
              setPhase("ready");
            }}
          />
        )}

        {phase === "ready" && me && (
          <MainLayout me={me} onLoggedOut={() => void refresh()} />
        )}
      </div>
    </div>
  );
}

function Ambient() {
  return (
    <div
      aria-hidden
      className="pointer-events-none absolute inset-0 overflow-hidden"
    >
      <div className="animate-blob absolute -top-40 -left-40 h-[520px] w-[520px] rounded-full bg-brand-500/10 blur-3xl" />
      <div className="animate-blob absolute top-1/3 -right-48 h-[560px] w-[560px] rounded-full bg-violet-500/10 blur-3xl [animation-delay:-6s]" />
      <div className="animate-blob absolute -bottom-56 left-1/4 h-[480px] w-[480px] rounded-full bg-sky-400/[0.07] blur-3xl [animation-delay:-3s]" />
    </div>
  );
}

function BootSplash() {
  return (
    <div className="flex h-full animate-fade-in flex-col items-center justify-center gap-5">
      <Logo size={64} />
      <div className="flex items-center gap-2 text-sm text-slate-400">
        <Spinner /> Conectando con Telegram…
      </div>
    </div>
  );
}

function CenteredCard({
  title,
  children,
}: {
  title: string;
  children: ReactNode;
}) {
  return (
    <div className="flex h-full items-center justify-center p-8">
      <Card className="w-full max-w-md animate-fade-up space-y-4">
        <h1 className="text-lg font-semibold text-white">{title}</h1>
        {children}
      </Card>
    </div>
  );
}
