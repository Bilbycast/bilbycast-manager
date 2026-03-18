use leptos::prelude::*;

/// A card displaying a single flow's statistics.
#[component]
pub fn FlowCard(
    name: &'static str,
    state: &'static str,
    input_type: &'static str,
    bitrate: &'static str,
    outputs: u32,
) -> impl IntoView {
    let state_color = match state {
        "Running" => "text-emerald-400",
        "Starting" => "text-blue-400",
        "Idle" | "Stopped" => "text-slate-400",
        _ => "text-red-400",
    };

    view! {
        <div class="bg-slate-750 rounded-lg border border-slate-700 p-4">
            <div class="flex items-center justify-between mb-3">
                <h4 class="text-sm font-medium text-white">{name}</h4>
                <span class=format!("text-xs font-medium {state_color}")>{state}</span>
            </div>
            <div class="grid grid-cols-3 gap-2 text-xs">
                <div>
                    <span class="text-slate-400">"Input: "</span>
                    <span class="text-white uppercase">{input_type}</span>
                </div>
                <div>
                    <span class="text-slate-400">"Rate: "</span>
                    <span class="text-white">{bitrate}</span>
                </div>
                <div>
                    <span class="text-slate-400">"Outputs: "</span>
                    <span class="text-white">{outputs.to_string()}</span>
                </div>
            </div>
        </div>
    }
}
