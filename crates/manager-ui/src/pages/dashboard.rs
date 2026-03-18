use leptos::prelude::*;

use crate::components::node_card::NodeCard;

/// Dashboard overview page showing all nodes and their status.
#[component]
pub fn DashboardPage() -> impl IntoView {
    view! {
        <div>
            // Page header
            <div class="mb-6">
                <h2 class="text-2xl font-bold text-white">"Dashboard"</h2>
                <p class="text-sm text-slate-400 mt-1">"Overview of all edge nodes"</p>
            </div>

            // Summary bar
            <div class="grid grid-cols-4 gap-4 mb-6">
                <SummaryCard label="Total Nodes" value="0" color="blue"/>
                <SummaryCard label="Online" value="0" color="green"/>
                <SummaryCard label="Offline" value="0" color="slate"/>
                <SummaryCard label="Active Alarms" value="0" color="red"/>
            </div>

            // Node grid
            <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                // Placeholder cards for now - will be populated from WebSocket data
                <NodeCard
                    name="Example Node 1"
                    status="online"
                    active_flows=3
                    total_flows=5
                    bitrate="45.2 Mbps"
                    uptime="2d 14h"
                />
                <NodeCard
                    name="Example Node 2"
                    status="offline"
                    active_flows=0
                    total_flows=2
                    bitrate="0 Mbps"
                    uptime="--"
                />
            </div>
        </div>
    }
}

#[component]
fn SummaryCard(label: &'static str, value: &'static str, color: &'static str) -> impl IntoView {
    let bg_class = match color {
        "blue" => "bg-blue-900/30 border-blue-800",
        "green" => "bg-emerald-900/30 border-emerald-800",
        "red" => "bg-red-900/30 border-red-800",
        _ => "bg-slate-800 border-slate-700",
    };

    view! {
        <div class=format!("rounded-lg border p-4 {bg_class}")>
            <p class="text-sm text-slate-400">{label}</p>
            <p class="text-2xl font-bold text-white mt-1">{value}</p>
        </div>
    }
}
