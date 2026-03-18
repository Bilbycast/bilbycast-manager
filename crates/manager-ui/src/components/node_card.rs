use leptos::prelude::*;

/// A card displaying an edge node's status in the dashboard grid.
#[component]
pub fn NodeCard(
    name: &'static str,
    status: &'static str,
    active_flows: u32,
    total_flows: u32,
    bitrate: &'static str,
    uptime: &'static str,
) -> impl IntoView {
    let status_color = match status {
        "online" => "bg-emerald-500",
        "degraded" => "bg-amber-500",
        "error" => "bg-red-500",
        _ => "bg-slate-500",
    };

    let status_text_color = match status {
        "online" => "text-emerald-400",
        "degraded" => "text-amber-400",
        "error" => "text-red-400",
        _ => "text-slate-400",
    };

    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-5 hover:border-slate-600 transition-colors cursor-pointer">
            // Header
            <div class="flex items-center justify-between mb-4">
                <div class="flex items-center space-x-3">
                    <div class=format!("w-2.5 h-2.5 rounded-full {status_color}")></div>
                    <h3 class="text-sm font-semibold text-white">{name}</h3>
                </div>
                <span class=format!("text-xs font-medium capitalize {status_text_color}")>{status}</span>
            </div>

            // Stats
            <div class="grid grid-cols-3 gap-3">
                <div>
                    <p class="text-xs text-slate-400">"Flows"</p>
                    <p class="text-sm font-medium text-white">{format!("{active_flows}/{total_flows}")}</p>
                </div>
                <div>
                    <p class="text-xs text-slate-400">"Bitrate"</p>
                    <p class="text-sm font-medium text-white">{bitrate}</p>
                </div>
                <div>
                    <p class="text-xs text-slate-400">"Uptime"</p>
                    <p class="text-sm font-medium text-white">{uptime}</p>
                </div>
            </div>
        </div>
    }
}
