use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(App);
}

#[component]
fn App() -> impl IntoView {
    view! {
        <main>
            <h1>"delinea"</h1>
            <p>"Camera-to-D2 live diagramming — scaffold. Camera capture, shape detection, and D2 rendering land in later milestones."</p>
        </main>
    }
}
