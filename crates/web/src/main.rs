use leptos::html::Video;
use leptos::prelude::*;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::MediaStreamConstraints;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    let video_ref = NodeRef::<Video>::new();
    let (error, set_error) = signal(None::<String>);

    let start_camera = move |_| {
        set_error.set(None);
        wasm_bindgen_futures::spawn_local(async move {
            match request_camera_stream().await {
                Ok(stream) => {
                    if let Some(video) = video_ref.get() {
                        video.set_src_object(Some(&stream));
                    }
                }
                Err(e) => set_error.set(Some(e)),
            }
        });
    };

    view! {
        <main>
            <h1>"delinea"</h1>
            <p>"Camera-to-D2 live diagramming — point a camera at a hand-drawn diagram."</p>
            <button on:click=start_camera>"Start camera"</button>
            {move || {
                error.get().map(|e| view! { <p style="color: red">{e}</p> })
            }}
            <video
                node_ref=video_ref
                autoplay=true
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
