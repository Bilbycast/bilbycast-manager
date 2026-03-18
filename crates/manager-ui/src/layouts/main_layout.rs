use leptos::prelude::*;
use leptos_router::components::Outlet;

/// Main layout with sidebar navigation and top bar.
/// Used for all authenticated pages.
#[component]
pub fn MainLayout() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-slate-900 text-slate-100 flex">
            // Sidebar
            <aside class="w-64 bg-slate-800 border-r border-slate-700 flex flex-col">
                // Logo
                <div class="p-4 border-b border-slate-700">
                    <h1 class="text-xl font-bold text-blue-400">"bilbycast"</h1>
                    <p class="text-xs text-slate-400">"manager"</p>
                </div>

                // Navigation
                <nav class="flex-1 p-4 space-y-1">
                    <NavItem href="/dashboard" label="Dashboard" icon="grid"/>
                    <NavItem href="/topology" label="Topology" icon="share-2"/>
                    <NavItem href="/events" label="Events" icon="bell"/>

                    <div class="pt-4 pb-2">
                        <p class="text-xs font-semibold text-slate-500 uppercase tracking-wider">"Administration"</p>
                    </div>
                    <NavItem href="/admin/users" label="Users" icon="users"/>
                    <NavItem href="/admin/settings" label="Settings" icon="settings"/>

                    <div class="pt-4 pb-2">
                        <p class="text-xs font-semibold text-slate-500 uppercase tracking-wider">"AI Assistant"</p>
                    </div>
                    <NavItem href="/ai/assistant" label="AI Config" icon="cpu"/>
                    <NavItem href="/ai/settings" label="AI Keys" icon="key"/>
                </nav>

                // Version
                <div class="p-4 border-t border-slate-700">
                    <p class="text-xs text-slate-500">"v0.1.0"</p>
                </div>
            </aside>

            // Main content area
            <div class="flex-1 flex flex-col">
                // Top bar
                <header class="h-14 bg-slate-800 border-b border-slate-700 flex items-center justify-between px-6">
                    <div class="flex items-center space-x-4">
                        <input
                            type="text"
                            placeholder="Search..."
                            class="bg-slate-700 text-sm text-slate-200 placeholder-slate-400 rounded-lg px-3 py-1.5 w-64 focus:outline-none focus:ring-2 focus:ring-blue-500"
                        />
                    </div>
                    <div class="flex items-center space-x-4">
                        // Alarm badge
                        <button class="relative p-2 text-slate-400 hover:text-white">
                            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"/>
                            </svg>
                            <span class="absolute -top-1 -right-1 bg-red-500 text-white text-xs rounded-full w-4 h-4 flex items-center justify-center">"0"</span>
                        </button>
                        // User menu
                        <button class="flex items-center space-x-2 text-slate-300 hover:text-white">
                            <div class="w-8 h-8 bg-blue-600 rounded-full flex items-center justify-center text-sm font-medium">"A"</div>
                            <span class="text-sm">"Admin"</span>
                        </button>
                    </div>
                </header>

                // Page content
                <main class="flex-1 overflow-auto p-6">
                    <Outlet/>
                </main>
            </div>
        </div>
    }
}

#[component]
fn NavItem(href: &'static str, label: &'static str, icon: &'static str) -> impl IntoView {
    let _ = icon; // Will use icon component later
    view! {
        <a
            href=href
            class="flex items-center space-x-3 px-3 py-2 rounded-lg text-sm text-slate-300 hover:bg-slate-700 hover:text-white transition-colors"
        >
            <span class="w-5 h-5 flex items-center justify-center text-slate-400">
                <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"/>
                </svg>
            </span>
            <span>{label}</span>
        </a>
    }
}
