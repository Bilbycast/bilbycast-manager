use leptos::prelude::*;

/// Events and alarms page with search, filter, and pagination.
#[component]
pub fn EventsPage() -> impl IntoView {
    view! {
        <div>
            <div class="mb-6">
                <h2 class="text-2xl font-bold text-white">"Events & Alarms"</h2>
                <p class="text-sm text-slate-400 mt-1">"Monitor and search system events across all nodes"</p>
            </div>

            // Filters
            <div class="bg-slate-800 rounded-xl border border-slate-700 p-4 mb-6">
                <div class="flex flex-wrap gap-4">
                    <select class="bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500">
                        <option value="">"All Severities"</option>
                        <option value="critical">"Critical"</option>
                        <option value="warning">"Warning"</option>
                        <option value="info">"Info"</option>
                    </select>
                    <select class="bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500">
                        <option value="">"All Nodes"</option>
                    </select>
                    <input
                        type="text"
                        placeholder="Search events..."
                        class="flex-1 min-w-[200px] bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                </div>
            </div>

            // Events table
            <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
                <table class="w-full">
                    <thead>
                        <tr class="border-b border-slate-700">
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Severity"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Time"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Node"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Category"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Message"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td colspan="6" class="px-4 py-8 text-center text-slate-400 text-sm">
                                "No events recorded yet"
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
    }
}
