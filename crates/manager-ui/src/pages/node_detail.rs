use leptos::prelude::*;

/// Node detail page showing stats mirroring the edge dashboard.
#[component]
pub fn NodeDetailPage() -> impl IntoView {
    view! {
        <div>
            <div class="mb-6 flex items-center justify-between">
                <div>
                    <h2 class="text-2xl font-bold text-white">"Node Detail"</h2>
                    <p class="text-sm text-slate-400 mt-1">"Real-time statistics and configuration"</p>
                </div>
                <div class="flex space-x-2">
                    <a
                        href="#"
                        class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors"
                    >
                        "Configure"
                    </a>
                </div>
            </div>

            // System info
            <div class="bg-slate-800 rounded-xl border border-slate-700 p-6 mb-6">
                <h3 class="text-lg font-semibold text-white mb-4">"System Information"</h3>
                <div class="grid grid-cols-4 gap-4">
                    <InfoItem label="Status" value="Online" value_class="text-emerald-400"/>
                    <InfoItem label="Version" value="0.1.0" value_class="text-white"/>
                    <InfoItem label="Uptime" value="2d 14h 32m" value_class="text-white"/>
                    <InfoItem label="Active Flows" value="3 / 5" value_class="text-white"/>
                </div>
            </div>

            // Flows section
            <div class="space-y-4">
                <h3 class="text-lg font-semibold text-white">"Flows"</h3>
                <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
                    <p class="text-slate-400 text-sm">"Connect this node to see real-time flow statistics"</p>
                </div>
            </div>
        </div>
    }
}

#[component]
fn InfoItem(label: &'static str, value: &'static str, value_class: &'static str) -> impl IntoView {
    view! {
        <div>
            <p class="text-xs text-slate-400 uppercase tracking-wider">{label}</p>
            <p class=format!("text-sm font-medium mt-1 {value_class}")>{value}</p>
        </div>
    }
}
