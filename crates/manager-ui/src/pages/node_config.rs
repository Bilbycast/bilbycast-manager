use leptos::prelude::*;

/// Node configuration page with config editor, flow management, and AI assistant.
#[component]
pub fn NodeConfigPage() -> impl IntoView {
    view! {
        <div>
            <div class="mb-6">
                <h2 class="text-2xl font-bold text-white">"Node Configuration"</h2>
                <p class="text-sm text-slate-400 mt-1">"View and edit node configuration, manage flows"</p>
            </div>

            // Full config editor
            <div class="bg-slate-800 rounded-xl border border-slate-700 p-6 mb-6">
                <div class="flex items-center justify-between mb-4">
                    <h3 class="text-lg font-semibold text-white">"Node Configuration"</h3>
                    <div class="flex space-x-2">
                        <button class="px-4 py-2 bg-slate-700 hover:bg-slate-600 text-white text-sm font-medium rounded-lg transition-colors">
                            "Refresh"
                        </button>
                        <button class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors">
                            "Save Configuration"
                        </button>
                    </div>
                </div>
                <p class="text-slate-400 text-sm">"Loading configuration from node..."</p>
            </div>

            // Flow list
            <div class="bg-slate-800 rounded-xl border border-slate-700 p-6 mb-6">
                <div class="flex items-center justify-between mb-4">
                    <h3 class="text-lg font-semibold text-white">"Flows"</h3>
                    <button class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors">
                        "New Flow"
                    </button>
                </div>
                <p class="text-slate-400 text-sm">"No flows configured. Create a new flow to get started."</p>
            </div>

            // IP Tunnels
            <div class="bg-slate-800 rounded-xl border border-slate-700 p-6 mb-6">
                <div class="flex items-center justify-between mb-4">
                    <h3 class="text-lg font-semibold text-white">"IP Tunnels"</h3>
                    <button class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors">
                        "New Tunnel"
                    </button>
                </div>
                <p class="text-slate-400 text-sm">"No tunnels configured. Create a TCP/UDP tunnel between nodes."</p>
            </div>

            // AI config generation
            <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
                <h3 class="text-lg font-semibold text-white mb-4">"AI Configuration Assistant"</h3>
                <div class="flex space-x-4">
                    <input
                        type="text"
                        placeholder="Describe the flow you want to create..."
                        class="flex-1 bg-slate-700 border border-slate-600 rounded-lg px-4 py-2.5 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                    <button class="px-6 py-2.5 bg-purple-600 hover:bg-purple-700 text-white text-sm font-medium rounded-lg transition-colors">
                        "Generate"
                    </button>
                </div>
                <p class="text-xs text-slate-500 mt-2">"Configure AI API keys in Settings to enable this feature"</p>
            </div>
        </div>
    }
}
