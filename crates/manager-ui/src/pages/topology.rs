use leptos::prelude::*;

/// Network topology view showing nodes and their interconnections.
#[component]
pub fn TopologyPage() -> impl IntoView {
    view! {
        <div>
            <div class="mb-6">
                <h2 class="text-2xl font-bold text-white">"Network Topology"</h2>
                <p class="text-sm text-slate-400 mt-1">"Visual map of node interconnections and stream routing"</p>
            </div>

            // Topology canvas placeholder
            <div class="bg-slate-800 rounded-xl border border-slate-700 p-8 min-h-[600px] flex items-center justify-center">
                <div class="text-center text-slate-400">
                    <svg class="w-16 h-16 mx-auto mb-4 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M13.828 10.172a4 4 0 00-5.656 0l-4 4a4 4 0 105.656 5.656l1.102-1.101m-.758-4.899a4 4 0 005.656 0l4-4a4 4 0 00-5.656-5.656l-1.1 1.1"/>
                    </svg>
                    <p class="text-lg font-medium">"Topology View"</p>
                    <p class="text-sm mt-2">"Connect edge nodes to see the network topology"</p>
                </div>
            </div>
        </div>
    }
}
