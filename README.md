# TeleDrive — nube ilimitada en tu Telegram

App de escritorio (Tauri 2 + Rust) que usa un **canal privado de Telegram como almacenamiento** y **Turso como índice**: subes archivos sin cuota, los trocea si son gigantes y los descargas verificados, sin servidores intermedios.

---

## Puesta en marcha

1. **Dependencias**

   ```powershell
   npm install
   ```

2. **Credenciales** — copia la plantilla y rellénala:

   ```powershell
   copy .env.example .env
   ```

   | Variable | Dónde sacarla |
   |---|---|
   | `TELEDRIVE_API_ID` | my.telegram.org → *API development tools* → `api_id` |
   | `TELEDRIVE_API_HASH` | my.telegram.org → *API development tools* → `api_hash` |
   | `TELEDRIVE_TURSO_URL` | `turso db show mi-nube --url` |
   | `TELEDRIVE_TURSO_TOKEN` | `turso db tokens create mi-nube` |

   Si no usas `.env`, la app te pregunta todo en pantalla y lo guarda en `config.json`.

3. **Base de datos de Turso** (una vez, con la CLI de Turso):

   ```powershell
   turso db create mi-nube
   ```

4. **Arrancar**

   ```powershell
   npm run tauri dev
   ```

   En el primer inicio: login con teléfono → código (llega a Telegram Web o por SMS) → 2FA si tienes. Después se crea solo el canal **«Mi Nube»** en tu cuenta.

---

## Comandos

| Comando | Dónde se ejecuta | Para qué |
|---|---|---|
| `npm run tauri dev` | raíz | App en modo desarrollo (recompila el backend al guardar) |
| `npm run tauri build` | raíz | Genera el instalador `.msi`/`.exe` en `src-tauri/target/release/bundle` |
| `npm run build` | raíz | Compila solo el frontend (`tsc` + `vite build`) |
| `npm run dev` | raíz | Solo el frontend en el navegador (sin backend Rust) |
| `cargo check` | `src-tauri/` | Comprueba el backend Rust |

---

## Qué hace

| Función | Detalle |
|---|---|
| Subida sin cuota | Trocea en partes de 1990 MB (3990 MB con Premium) y sube cada parte al canal |
| Deduplicado | SHA-256 local: si el mismo contenido ya está en tu nube, no se vuelve a subir |
| Descarga | Reensambla las partes, verifica el SHA-256 y descarta el archivo si no coincide |
| Índice en Turso | Búsqueda por nombre, favoritos, papelera, «Recientes» y miniaturas |
| Reconstrucción | *Sincronizar* relee los manifiestos del canal y rehace el índice si lo pierdes |
| Sesión local | La sesión de Telegram (`session.sqlite`) nunca sale de tu PC |

---

## Dónde viven tus datos

| Dato | Ubicación |
|---|---|
| Archivos | Tu canal privado «Mi Nube» en Telegram |
| Índice (nombres, tamaños, estado) | Tu base de datos de Turso |
| Credenciales de la app | `%APPDATA%\com.teledrive.app\config.json` |
| Sesión de Telegram | `%APPDATA%\com.teledrive.app\session.sqlite` |
| `.env` (opcional) | Raíz del proyecto o carpeta del ejecutable |

---

## Seguridad: qué nunca se sube a Git

`.gitignore` ya bloquea credenciales y datos locales:

| Ignorado | Motivo |
|---|---|
| `.env` / `.env.*` (salvo `.env.example`) | API ID/hash y token de Turso |
| `config.json` | Copia local de credenciales |
| `*.sqlite` | Sesión de Telegram (es tu llave de acceso) |
| `node_modules`, `dist`, `src-tauri/target` | Dependencias y builds |

Checklist antes de publicar:

- [ ] `.env` no aparece en `git status` (solo `.env.example`)
- [ ] No hay tokens de Turso ni `api_hash` en el código
- [ ] `session.sqlite` y `config.json` no están en el repo

Si algo se filtró, **rótalo**: regenera el token con `turso db tokens create mi-nube`, cambia el `api_hash` en my.telegram.org y borra `session.sqlite` para forzar un login nuevo.

---

## Estructura del proyecto

| Ruta | Contenido |
|---|---|
| `src/` | UI React 19 + Tailwind v4 (`components/`, `lib/api.ts`) |
| `src-tauri/src/telegram/` | `auth`, `drive`, `upload`, `download`, `sync`, `preview` |
| `src-tauri/src/turso.rs` | Esquema y consultas del índice remoto |
| `src-tauri/src/manifest.rs` | Manifiesto `td1|` que viaja en el caption de cada parte |
| `src-tauri/src/envfile.rs` | Lectura del `.env` |
| `src-tauri/src/state.rs` | Configuración, sesión y cliente de Telegram |

---

## Límites conocidos

- **Telegram Desktop no hace falta**: el login es por teléfono/código/2FA y el código llega a Telegram Web.
- Una parte máximo 1990 MB (3990 MB con Premium).
- La sincronización lee hasta 2000 mensajes del canal por pasada.
- `glass_pumpkin` está fijado a `=2.0.0-rc0`: la rc1 rompe `grammers-crypto 0.10.0`. No lo actualices.
- En Windows, `cargo test` puede fallar con *error 1455* (pagefile insuficiente al enlazar); no afecta al build normal.

---

## Siguientes pasos

- [ ] Carpetas reales (tabla `folders` + selector al subir)
- [ ] Cifrado opcional antes de subir
- [ ] Compartir por enlace con caducidad
- [ ] Varias cuentas de Telegram en la misma app
