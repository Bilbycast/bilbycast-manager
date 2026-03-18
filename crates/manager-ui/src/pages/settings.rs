use leptos::prelude::*;

/// System settings page.
#[component]
pub fn SettingsPage() -> impl IntoView {
    view! {
        <div>
            <div class="mb-6">
                <h2 class="text-2xl font-bold text-white">"System Settings"</h2>
                <p class="text-sm text-slate-400 mt-1">"Configure server behavior and data retention"</p>
            </div>

            <div class="space-y-6">
                // Data Retention
                <SettingsSection title="Data Retention">
                    <SettingField
                        label="Event retention (days)"
                        description="How long to keep events before automatic cleanup"
                        input_type="number"
                        value="30"
                    />
                </SettingsSection>

                // WebSocket
                <SettingsSection title="WebSocket">
                    <SettingField
                        label="Keepalive interval (seconds)"
                        description="Ping interval for node WebSocket connections"
                        input_type="number"
                        value="15"
                    />
                    <SettingField
                        label="Node offline threshold (seconds)"
                        description="Seconds without heartbeat before marking a node offline"
                        input_type="number"
                        value="30"
                    />
                    <SettingField
                        label="Stats broadcast interval (ms)"
                        description="How often aggregated stats are pushed to browser clients"
                        input_type="number"
                        value="1000"
                    />
                </SettingsSection>

                // Security
                <SettingsSection title="Security">
                    <SettingField
                        label="Session lifetime (hours)"
                        description="How long a user session remains valid"
                        input_type="number"
                        value="24"
                    />
                    <SettingField
                        label="Max login attempts per minute"
                        description="Rate limit for login endpoint per IP"
                        input_type="number"
                        value="5"
                    />
                </SettingsSection>

                // Export/Import
                <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
                    <h3 class="text-lg font-semibold text-white mb-4">"Backup & Restore"</h3>
                    <div class="flex space-x-4">
                        <button class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors">
                            "Export All Data"
                        </button>
                        <button class="px-4 py-2 bg-slate-600 hover:bg-slate-500 text-white text-sm font-medium rounded-lg transition-colors">
                            "Import Data"
                        </button>
                    </div>
                    <p class="text-xs text-slate-500 mt-2">"Export includes users, nodes, settings, and optionally events"</p>
                </div>

                // Save button
                <div class="flex justify-end">
                    <button class="px-6 py-2.5 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-lg transition-colors">
                        "Save Settings"
                    </button>
                </div>
            </div>
        </div>
    }
}

#[component]
fn SettingsSection(title: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <h3 class="text-lg font-semibold text-white mb-4">{title}</h3>
            <div class="space-y-4">
                {children()}
            </div>
        </div>
    }
}

#[component]
fn SettingField(
    label: &'static str,
    description: &'static str,
    input_type: &'static str,
    value: &'static str,
) -> impl IntoView {
    view! {
        <div class="flex items-center justify-between">
            <div>
                <p class="text-sm font-medium text-slate-200">{label}</p>
                <p class="text-xs text-slate-400">{description}</p>
            </div>
            <input
                type=input_type
                value=value
                class="w-24 bg-slate-700 border border-slate-600 rounded-lg px-3 py-1.5 text-sm text-white text-right focus:outline-none focus:ring-2 focus:ring-blue-500"
            />
        </div>
    }
}
