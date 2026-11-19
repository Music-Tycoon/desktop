# Music Tycoon de escritorio (Tauri v2)

Music Tycoon como app de escritorio (macOS, Windows, Linux) con [Tauri v2](https://v2.tauri.app). No hay una segunda
interfaz: es **la misma app web** (`../musicopoly-app`) compilada y metida dentro de la app, en una ventana con el
navegador del sistema (WKWebView en macOS, WebView2 en Windows, WebKitGTK en Linux). Pesa poco porque no lleva un
Chromium dentro (Electron sí).

```
musicopoly-desktop/
├─ package.json            scripts (build:web, build, build:debug, icons) y @tauri-apps/cli
├─ scripts/build-web.mjs   compila ../musicopoly-app en ./web-dist (no toca musicopoly-app/dist)
├─ web-dist/               (generado) la app web que va dentro
└─ src-tauri/
   ├─ tauri.conf.json      nombre, icono, paquete, esquema musicopoly://
   ├─ src/lib.rs           ventana, servidor online, enlaces de invitación, diagnóstico
   ├─ Info.plist           textos de micrófono y cámara (macOS) + idiomas
   ├─ macos/{es,en}.lproj  textos de permisos traducidos
   ├─ Entitlements.plist   micrófono y cámara con Hardened Runtime (para notarizar)
   └─ icons/               generados desde brand/out/mark-1024.png (npm run icons)
```

## Cómo funciona

- **La interfaz se sirve en `http://localhost:47821`** (plugin `tauri-plugin-localhost`), no en `tauri://localhost`.
  Motivos: YouTube rechaza sus vídeos incrustados (error 152/153) si la página no es http(s) y no manda Referer, y
  `localhost` es un «contexto seguro», así que micrófono, cámara y WebRTC funcionan como en la web.
  El puerto es fijo: el servidor lo admite en su CORS (`musicopoly-server/src/api.ts`, `DESKTOP_ORIGINS`) y el
  almacenamiento local (perfil, partidas guardadas) va ligado a ese origen.
- **Servidor online configurable.** La app web ya no supone que el servidor es el mismo origen: lee
  `window.__MUSICOPOLY_SERVER__` (lo inyecta la app al arrancar) o `VITE_SERVER_URL` al compilar
  (`musicopoly-app/src/net/server.ts`). En la web no cambia nada: sin configurar, todo va al mismo origen.
  Orden de prioridad en la app de escritorio:
  1. variable de entorno al abrir: `MUSICOPOLY_SERVER=https://mi-servidor open "Music Tycoon.app"`
  2. archivo `config.json` en la carpeta de configuración de la app
     (macOS: `~/Library/Application Support/app.musicopoly.desktop/config.json`) con `{"server": "https://…"}`
  3. el indicado al compilar: `MUSICOPOLY_SERVER=https://… npm run build`
- **Invitaciones.** Los enlaces que copia la app de escritorio apuntan a la web pública (`https://servidor/?sala=ABCD`),
  así que el invitado puede entrar desde el navegador. Además la app abre `musicopoly://sala/ABCD`
  (plugin `deep-link`): la ventana va directa a «Te invitan a jugar».
- **Sin service worker** en la app (los archivos ya van dentro); en la web sigue igual.

## Compilar y probar

Requisitos: Node 20+, Rust (rustup) y, en macOS, Xcode (o Command Line Tools). En Windows: Visual Studio Build Tools
(C++) y WebView2 (ya viene con Windows 10/11). En Linux: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `librsvg2-dev`…
(ver [requisitos](https://v2.tauri.app/start/prerequisites/)).

```bash
cd musicopoly-desktop
npm install

# app de depuración (rápida de compilar, con inspector web: clic derecho → Inspeccionar)
MUSICOPOLY_SERVER=https://tu-servidor npm run build:debug -- --bundles app
open "src-tauri/target/debug/bundle/macos/Music Tycoon.app"

# versión final (optimizada; en macOS crea .app y .dmg; en Windows .msi y .exe; en Linux .deb/.rpm/.AppImage)
MUSICOPOLY_SERVER=https://tu-servidor npm run build

# desarrollo con recarga en caliente: abre la ventana sobre el Vite de musicopoly-app (http://localhost:5181)
npm run dev
```

Diagnóstico (solo compilaciones de depuración): `MUSICOPOLY_DIAG=1` (o `=yt`, que además pone un vídeo de YouTube
encima) hace que la propia ventana compruebe contexto seguro, getUserMedia, WebRTC, WebSocket y API, y lo imprima:

```bash
MUSICOPOLY_DIAG=yt src-tauri/target/debug/bundle/macos/Music\ Tycoon.app/Contents/MacOS/musicopoly-desktop
# → MUSICOPOLY_DIAG {"origin":"http://localhost:47821","secure":true,"ws":"open","api":200,"dataChannel":"hola",…}
```

Al servidor solo le hace falta admitir el origen `http://localhost:47821` en el CORS de `/api` (ya añadido en
`api.ts`; también vale `MUSICOPOLY_ORIGINS=http://localhost:47821`). El WebSocket de las salas no lo necesita.

## Probado (macOS, app de depuración, 17-11-2026)

- La app abre la ventana «Music Tycoon» (1400×900, mínimo 960×640) y pinta la portada desde `http://localhost:47821`.
- Desde dentro de WKWebView: `isSecureContext` = sí, `getUserMedia` disponible, `RTCPeerConnection` con canal de
  datos local funcionando («hola» de ida y vuelta), WebSocket al servidor online por el túnel abierto.
- **YouTube:** reproductor con la API de YouTube (como en la partida): `ready` → reproduciendo (estado 1, 6,5 s),
  sin error 152/153.
- API `/api` desde la app: 200 con el CORS nuevo (en un servidor de prueba); el servidor publicado aún responde
  «bloqueado» hasta que se reinicie con `api.ts` actualizado.
- Sala creada desde la interfaz que sirve la app (con el servidor configurado): el enlace de invitación sale con la
  URL del servidor y un invitado desde el navegador entra en la misma sala.
- `open musicopoly://sala/pdxt` con la app abierta → la ventana pasa a `/?sala=PDXT`.
- Info.plist con los textos de micrófono/cámara y `es.lproj`/`en.lproj` dentro del paquete; esquema `musicopoly`
  registrado. Sin probar aquí: el diálogo real de permisos (no se lanzó para no molestar en este equipo), Windows y Linux.

## Qué funciona (y límites por sistema)

| | macOS (WKWebView) | Windows (WebView2 = Chromium) | Linux (WebKitGTK) |
|---|---|---|---|
| Jugar aquí (mismo dispositivo) | sí | sí | sí |
| Online con amigos (WebSocket) | sí | sí | sí |
| Voz y vídeo (WebRTC + getUserMedia) | sí (pide permiso de macOS con nuestros textos) | sí (diálogo de permiso de WebView2) | **no** en la mayoría de distros: WebKitGTK viene sin WebRTC |
| Sin internet, con QR (WebRTC DataChannel + cámara) | sí | sí | **no** (mismo motivo) |
| Vídeos de YouTube | sí, gracias a `http://localhost` | sí | depende de los códecs de GStreamer instalados |
| Spotify incrustado | sí (sus límites de siempre: 30 s sin sesión) | sí | sí, con los mismos códecs |
| Enlaces `musicopoly://` | sí (con la app en /Aplicaciones) | sí (se registra al instalar) | sí (.deb/.rpm; AppImage necesita registrarlo) |

Otros límites conocidos:
- **macOS:** el permiso de micrófono/cámara lo da el sistema (Ajustes → Privacidad). En una app sin firmar
  (o firmada «ad-hoc») macOS puede volver a preguntar tras cada versión. WKWebView es Safari: lo que falle en Safari
  (p. ej. reproducción automática con sonido sin un toque) falla igual aquí.
- **Windows:** WebView2 viene con Windows 10/11; el instalador lleva el «bootstrapper» por si falta (equipos viejos).
- **Linux:** sin WebRTC no hay voz/vídeo ni mesa sin internet. Alternativa: dejar Linux en la web (navegador).
- **Almacenamiento:** la app de escritorio tiene su propio almacenamiento (no comparte sesión, perfil ni partidas
  guardadas con el navegador). La cuenta se recupera iniciando sesión.
- **Puerto 47821:** si otro programa lo usa, la app no puede abrir la interfaz. Dos copias abiertas a la vez en macOS
  chocarían por el puerto (en Windows/Linux lo evita `single-instance`).

## Firma, notarización y costes

Sin firmar, la app funciona pero cada sistema avisa: macOS la bloquea («no se puede abrir porque es de un
desarrollador no identificado»; hay que ir a Ajustes → Privacidad → «Abrir igualmente») y Windows muestra SmartScreen.

**macOS (Apple Developer Program, 99 USD/año):**
1. Darse de alta en el programa (persona o empresa; empresa necesita número D-U-N-S).
2. Crear un certificado **Developer ID Application** (solo el titular de la cuenta) e instalarlo en el llavero.
3. Compilar con firma y notarización (Tauri lo hace solo con estas variables):
   ```bash
   export APPLE_SIGNING_IDENTITY="Developer ID Application: Nombre (TEAMID)"
   export APPLE_API_ISSUER=… APPLE_API_KEY=… APPLE_API_KEY_PATH=…/AuthKey_XXXX.p8   # clave de App Store Connect
   # (o bien APPLE_ID, APPLE_PASSWORD = contraseña de app, APPLE_TEAM_ID)
   npm run build
   ```
   `Entitlements.plist` ya incluye micrófono y cámara para el Hardened Runtime.
4. Universal (Intel + Apple Silicon): `npm run build -- --target universal-apple-darwin`.

**Windows:** firmar reduce SmartScreen (la reputación se gana con descargas, también con firma).
- **Azure Artifact Signing** (antes «Trusted Signing»): la opción más barata y sin token físico,
  ~10 USD/mes; requiere verificación de identidad (empresas y, en algunos países, particulares).
  En `tauri.conf.json` → `bundle.windows.signCommand`.
- Certificado **OV** de una CA (~200–400 USD/año; desde 2023 va en token USB/HSM) o **EV** (~300–600 USD/año). Desde
  2024 EV ya no da reputación inmediata en SmartScreen, así que no compensa frente a OV/Azure.
- Si se publica solo en la **Microsoft Store**, la Store firma el paquete y no hace falta certificado propio.

**Linux:** no hay firma obligatoria (opcional: firmar el AppImage o el repositorio .deb con GPG).

## Actualizaciones automáticas (siguiente paso)

`tauri-plugin-updater`: la app consulta un `latest.json` (p. ej. en el propio servidor o en GitHub Releases),
descarga el paquete nuevo, comprueba su firma y se reinicia.
1. `npx tauri signer generate -w ~/.tauri/musicopoly.key` (guarda la clave privada fuera del repositorio).
2. En `tauri.conf.json`: `"bundle": {"createUpdaterArtifacts": true}` y
   `"plugins": {"updater": {"pubkey": "…", "endpoints": ["https://servidor/desktop/latest.json"]}}`.
3. Compilar con `TAURI_SIGNING_PRIVATE_KEY` y subir los `.tar.gz`/`.sig` + `latest.json`.
En macOS la actualización solo es fluida si la app está firmada y notarizada.

Alternativa sin actualizador: que la ventana cargue directamente la web publicada (`WebviewUrl::External` con la
URL del servidor). Siempre al día, pero sin internet no abre (no habría «Jugar aquí» ni mesa sin internet sin red).

## Distribución

| Opción | A favor | En contra |
|---|---|---|
| **Descarga directa** (web/landing: .dmg, .msi/.exe, .AppImage/.deb) | Control total; cobro del Club con nuestro propio pago; actualizaciones con el updater; sin revisión | Hay que firmar (99 USD/año Apple + firma Windows) para evitar avisos; hosting de descargas |
| **Mac App Store** | Confianza, descubrimiento, actualizaciones gratis | Sandbox obligatorio (más entitlements: red cliente y servidor local); revisión (riesgo «solo envuelve una web», guía 4.2); **pagos digitales (Club) por compra integrada de Apple con comisión 15–30 %** |
| **Microsoft Store** | Alta gratuita para particulares; la Store firma el paquete (sin certificado); sin SmartScreen | Revisión; para juegos, los contenidos digitales suelen tener que ir por el sistema de compras de Microsoft — revisar la política vigente antes de vender el Club ahí |
| **Steam** | Público jugador; logros/amigos | 100 USD por juego, 30 % de comisión, integrar Steamworks; el pago del Club por fuera no está permitido |
| **Flathub / Snap (Linux)** | Fácil de instalar | Sin WebRTC en WebKitGTK: experiencia recortada |

Recomendación: empezar con **descarga directa** en macOS y Windows (firmado + updater) desde la landing, mantener
Linux y móviles en la web, y valorar las tiendas cuando el modelo de cobro del Club esté decidido.

## Tamaño

- Interfaz (`web-dist`): ~27 MB, casi todo imágenes de logos/arte (12 MB + 9 MB) y muestras de audio (1,7 MB).
- Binario Rust de la ventana en versión final: ~8–12 MB. Instalador final esperado: ~25–35 MB comprimido.
  (Una app Electron equivalente pasaría de 150 MB.) Bajar las imágenes a WebP/AVIF reduciría bastante más.
