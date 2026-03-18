use leptos::prelude::*;
use leptos_router::components::Outlet;

/// Auth layout for login page (minimal, centered).
#[component]
pub fn AuthLayout() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-slate-900 flex items-center justify-center">
            <Outlet/>
        </div>
    }
}
