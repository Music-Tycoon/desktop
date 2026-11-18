//! Musicopoly de escritorio: una ventana con la app web (la misma de musicopoly-app, compilada en ../web-dist)
//! servida en http://localhost:47821 y conectada al servidor de salas online.
//!
//! Servidor: 1) variable de entorno MUSICOPOLY_SERVER al abrir la app, 2) `config.json` con {"server": "https://…"}
//! en la carpeta de configuración de la app, 3) el que se indicó al compilar (MUSICOPOLY_SERVER=… npm run build).
//! La página lo recibe como `window.__MUSICOPOLY_SERVER__` (ver musicopoly-app/src/net/server.ts).

use tauri::{webview::PageLoadEvent, AppHandle, Manager, Url, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_deep_link::DeepLinkExt;

/// Puerto de la interfaz. Fijo: el servidor lo admite en su CORS (musicopoly-server/src/api.ts, DESKTOP_ORIGINS)
/// y el almacenamiento local (perfil, partidas guardadas) va ligado al origen.
const PORT: u16 = 47821;
const BUILD_SERVER: Option<&str> = option_env!("MUSICOPOLY_SERVER");

fn server_url(app: &AppHandle) -> Option<String> {
    let clean = |s: &str| {
        let s = s.trim().trim_end_matches('/');
        (s.starts_with("http://") || s.starts_with("https://")).then(|| s.to_string())
    };
    if let Some(s) = std::env::var("MUSICOPOLY_SERVER").ok().as_deref().and_then(clean) {
        return Some(s);
    }
    if let Ok(dir) = app.path().app_config_dir() {
        if let Ok(txt) = std::fs::read_to_string(dir.join("config.json")) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&txt) {
                if let Some(s) = json.get("server").and_then(|v| v.as_str()).and_then(clean) {
                    return Some(s);
                }
            }
        }
    }
    BUILD_SERVER.and_then(clean)
}

/// musicopoly://sala/ABCD, musicopoly://ABCD o https://…/?sala=ABCD → «ABCD»
fn room_code(url: &Url) -> Option<String> {
    let raw = url.query_pairs().find(|(k, _)| k == "sala").map(|(_, v)| v.to_string()).or_else(|| {
        let mut parts: Vec<String> = url.host_str().map(|h| vec![h.to_string()]).unwrap_or_default();
        parts.extend(url.path().split('/').filter(|s| !s.is_empty()).map(String::from));
        parts.into_iter().filter(|p| !p.eq_ignore_ascii_case("sala") && !p.eq_ignore_ascii_case("room")).last()
    })?;
    let code: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect::<String>().to_uppercase();
    (3..=8).contains(&code.len()).then_some(code)
}

/// Llega un enlace de invitación con la app abierta: la ventana va a la pantalla «Te invitan a jugar»
fn open_invite(app: &AppHandle, urls: &[Url]) {
    let Some(code) = urls.iter().find_map(room_code) else { return };
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.eval(&format!("location.href = '/?sala={code}'"));
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        // segunda instancia (p. ej. al abrir un enlace musicopoly://): se avisa a la primera y se cierra
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            let urls: Vec<Url> = argv.iter().filter_map(|a| Url::parse(a).ok()).collect();
            open_invite(app, &urls);
        }));
    }
    builder
        .plugin(tauri_plugin_localhost::Builder::new(PORT).build())
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let handle = app.handle().clone();
            let server = server_url(&handle);
            // la app abierta con un enlace de invitación entra directamente en esa sala
            let start = app.deep_link().get_current().ok().flatten().and_then(|u| u.iter().find_map(room_code));
            let query = start.map(|c| format!("?sala={c}")).unwrap_or_default();
            let url = if tauri::is_dev() {
                WebviewUrl::App(format!("index.html{query}").into())
            } else {
                WebviewUrl::External(format!("http://localhost:{PORT}/{query}").parse()?)
            };
            let init = format!(
                "window.__MUSICOPOLY_DESKTOP__ = true;{}",
                server.as_ref().map(|s| format!("window.__MUSICOPOLY_SERVER__ = {};", serde_json::Value::from(s.as_str()))).unwrap_or_default()
            );
            let win = WebviewWindowBuilder::new(app, "main", url)
                .title("Musicopoly")
                .inner_size(1400.0, 900.0)
                .min_inner_size(960.0, 640.0)
                .center()
                .initialization_script(&init);
            #[cfg(debug_assertions)]
            let win = diag::attach(win);
            win.build()?;

            #[cfg(any(windows, target_os = "linux"))]
            {
                // en Windows/Linux el esquema se registra al instalar; así también funciona en desarrollo
                let _ = app.deep_link().register_all();
            }
            app.deep_link().on_open_url(move |event| open_invite(&handle, &event.urls()));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("no se pudo abrir Musicopoly");
}

/// Solo en compilaciones de depuración y con MUSICOPOLY_DIAG=1: comprueba desde la propia ventana el contexto
/// seguro, getUserMedia, WebRTC (canal de datos en local), WebSocket y API del servidor, y lo imprime por consola
/// (la página «navega» a http://diag.invalid/?… y aquí se intercepta). Con MUSICOPOLY_DIAG=yt, además pone encima
/// un vídeo de YouTube incrustado para ver si carga (error 152/153).
#[cfg(debug_assertions)]
mod diag {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    static DONE: AtomicBool = AtomicBool::new(false);

    const SCRIPT: &str = r#"(async () => {
      const S = window.__MUSICOPOLY_SERVER__ || '';
      const r = { origin: location.origin, secure: isSecureContext, server: S, ua: navigator.userAgent,
        getUserMedia: !!(navigator.mediaDevices && navigator.mediaDevices.getUserMedia), rtc: typeof RTCPeerConnection,
        serviceWorkerApi: 'serviceWorker' in navigator, appRendered: !!document.querySelector('#root > *') };
      const wait = (p, ms) => Promise.race([p, new Promise((res) => setTimeout(() => res('timeout'), ms))]);
      try {
        r.ws = await wait(new Promise((res) => { const s = new WebSocket(S.replace(/^http/, 'ws') + '/ws'); s.onopen = () => { s.close(); res('open'); }; s.onerror = () => res('error'); }), 8000);
      } catch (e) { r.ws = String(e); }
      try { const x = await wait(fetch(S + '/api/leaderboard'), 8000); r.api = x.status ?? x; } catch (e) { r.api = 'blocked: ' + e; }
      try {
        const a = new RTCPeerConnection(), b = new RTCPeerConnection();
        a.onicecandidate = (e) => e.candidate && b.addIceCandidate(e.candidate);
        b.onicecandidate = (e) => e.candidate && a.addIceCandidate(e.candidate);
        const got = new Promise((res) => (b.ondatachannel = (e) => (e.channel.onmessage = (m) => res(m.data))));
        const ch = a.createDataChannel('t'); ch.onopen = () => ch.send('hola');
        await a.setLocalDescription(await a.createOffer()); await b.setRemoteDescription(a.localDescription);
        await b.setLocalDescription(await b.createAnswer()); await a.setRemoteDescription(b.localDescription);
        r.dataChannel = await wait(got, 8000);
      } catch (e) { r.dataChannel = 'error: ' + e; }
      location.href = 'http://diag.invalid/?' + encodeURIComponent(JSON.stringify(r));
      if (DIAG_YT) {
        // reproductor de YouTube con su API (como Music.tsx): ¿carga, da error (152/153 = sin Referer) o suena?
        const y = { events: [] };
        const box = document.createElement('div');
        box.style.cssText = 'position:fixed;z-index:99999;right:24px;bottom:24px;width:480px;height:270px';
        box.innerHTML = '<div id="diag-yt"></div>';
        document.body.appendChild(box);
        await new Promise((res) => {
          window.onYouTubeIframeAPIReady = res;
          const s = document.createElement('script'); s.src = 'https://www.youtube.com/iframe_api'; s.onerror = () => res('noapi'); document.head.appendChild(s);
          setTimeout(res, 10000);
        });
        if (window.YT && YT.Player) {
          await new Promise((res) => {
            const p = new YT.Player('diag-yt', { width: 480, height: 270, videoId: 'fJ9rUzIMcZQ', playerVars: { origin: location.origin, playsinline: 1 },
              events: {
                onReady: () => { y.events.push('ready'); p.mute(); p.playVideo(); setTimeout(() => { y.state = p.getPlayerState(); y.time = p.getCurrentTime(); res(); }, 7000); },
                onError: (e) => { y.events.push('error ' + e.data); res(); },
                onStateChange: (e) => y.events.push('state ' + e.data),
              } });
            setTimeout(res, 20000);
          });
        } else y.events.push('no YT api');
        location.href = 'http://diag.invalid/?' + encodeURIComponent(JSON.stringify({ youtube: y }));
      }
    })();"#;

    pub fn attach<'a, R: tauri::Runtime, M: Manager<R>>(win: WebviewWindowBuilder<'a, R, M>) -> WebviewWindowBuilder<'a, R, M> {
        let Ok(mode) = std::env::var("MUSICOPOLY_DIAG") else { return win };
        let script = SCRIPT.replace("DIAG_YT", if mode == "yt" { "true" } else { "false" });
        win.on_navigation(|url| {
            if url.host_str() == Some("diag.invalid") {
                println!("MUSICOPOLY_DIAG {}", url.query().map(|q| percent_decode(q)).unwrap_or_default());
                return false;
            }
            true
        })
        .on_page_load(move |w, p| {
            if p.event() == PageLoadEvent::Finished {
                println!("MUSICOPOLY_PAGE {}", p.url());
            }
            if p.event() == PageLoadEvent::Finished && !DONE.swap(true, Ordering::SeqCst) {
                let w = w.clone();
                let script = script.clone();
                // un momento para que React pinte la portada
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                    let _ = w.eval(&script);
                });
            }
        })
    }

    fn percent_decode(s: &str) -> String {
        let b = s.as_bytes();
        let mut out = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'%' && i + 2 < b.len() {
                if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                    out.push(v);
                    i += 3;
                    continue;
                }
            }
            out.push(b[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).into_owned()
    }
}
