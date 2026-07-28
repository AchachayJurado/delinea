use leptos::html::Video;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::MediaStreamConstraints;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[derive(Clone, PartialEq)]
enum CameraState {
    Idle,
    Requesting,
    Active,
    Error(String),
}

impl CameraState {
    fn status_text(&self) -> String {
        match self {
            CameraState::Idle => "idle — click \"Start camera\"".to_string(),
            CameraState::Requesting => {
                "requesting camera access… look for a permission prompt".to_string()
            }
            CameraState::Active => "camera active".to_string(),
            CameraState::Error(e) => format!("error — {e}"),
        }
    }
}

#[component]
fn App() -> impl IntoView {
    let video_ref = NodeRef::<Video>::new();
    let (state, set_state) = signal(CameraState::Idle);

    let start_camera = move |_| {
        set_state.set(CameraState::Requesting);
        wasm_bindgen_futures::spawn_local(async move {
            let Some(video) = video_ref.get_untracked() else {
                set_state.set(CameraState::Error(
                    "video element not ready — reload the page and try again".to_string(),
                ));
                return;
            };

            let stream = match request_camera_stream().await {
                Ok(stream) => stream,
                Err(e) => {
                    set_state.set(CameraState::Error(e));
                    return;
                }
            };

            video.set_src_object(Some(&stream));
            let play_result = match video.play() {
                Ok(promise) => JsFuture::from(promise).await,
                Err(e) => Err(e),
            };
            if let Err(e) = play_result {
                set_state.set(CameraState::Error(format!(
                    "camera stream attached but playback failed: {e:?}"
                )));
                return;
            }

            set_state.set(CameraState::Active);
        });
    };

    view! {
        <main>
            <h1>"delinea"</h1>
            <p>"Camera-to-D2 live diagramming — point a camera at a hand-drawn diagram."</p>
            <button on:click=start_camera>"Start camera"</button>
            <p>"Status: " {move || state.get().status_text()}</p>
            <video
                node_ref=video_ref
                autoplay=true
                muted=true
                playsinline=true
                style="max-width: 640px; display: block; margin-top: 1rem; background: #222;"
            ></video>
        </main>
    }
}

async fn request_camera_stream() -> Result<web_sys::MediaStream, String> {
    let window = web_sys::window().ok_or("no window available")?;
    let media_devices = window
        .navigator()
        .media_devices()
        .map_err(|e| format!("media devices unavailable: {e:?}"))?;

    let constraints = MediaStreamConstraints::new();
    constraints.set_video(&JsValue::TRUE);

    let promise = media_devices
        .get_user_media_with_constraints(&constraints)
        .map_err(|e| format!("getUserMedia failed: {e:?}"))?;

    JsFuture::from(promise)
        .await
        .map_err(|e| format!("camera permission denied or unavailable: {e:?}"))?
        .dyn_into::<web_sys::MediaStream>()
        .map_err(|_| "unexpected stream type".to_string())
}
