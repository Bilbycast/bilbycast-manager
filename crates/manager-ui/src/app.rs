use leptos::prelude::*;
use leptos_meta::*;
use leptos_router::{components::*, path};

use crate::layouts::main_layout::MainLayout;
use crate::layouts::auth_layout::AuthLayout;
use crate::pages::*;

/// Root Leptos application component with routing.
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    view! {
        <Stylesheet href="/style/output.css"/>
        <Title text="bilbycast-manager"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1.0"/>

        <Router>
            <Routes fallback=|| view! { <p class="text-white p-8">"404 - Page not found"</p> }>
                // Auth routes (no sidebar)
                <ParentRoute path=path!("/login") view=AuthLayout>
                    <Route path=path!("") view=login::LoginPage/>
                </ParentRoute>

                // Main authenticated routes
                <ParentRoute path=path!("/") view=MainLayout>
                    <Route path=path!("") view=|| view! { <leptos_router::components::Redirect path="/dashboard"/> }/>
                    <Route path=path!("dashboard") view=dashboard::DashboardPage/>
                    <Route path=path!("topology") view=topology::TopologyPage/>
                    <Route path=path!("nodes/:node_id") view=node_detail::NodeDetailPage/>
                    <Route path=path!("nodes/:node_id/config") view=node_config::NodeConfigPage/>
                    <Route path=path!("events") view=events::EventsPage/>
                    <Route path=path!("admin/users") view=users::UsersPage/>
                    <Route path=path!("admin/settings") view=settings::SettingsPage/>
                    <Route path=path!("ai/assistant") view=ai_assistant::AiAssistantPage/>
                    <Route path=path!("ai/settings") view=ai_settings::AiSettingsPage/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}
