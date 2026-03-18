use leptos::prelude::*;

/// User management page.
#[component]
pub fn UsersPage() -> impl IntoView {
    view! {
        <div>
            <div class="mb-6 flex items-center justify-between">
                <div>
                    <h2 class="text-2xl font-bold text-white">"User Management"</h2>
                    <p class="text-sm text-slate-400 mt-1">"Manage users, roles, and access permissions"</p>
                </div>
                <button class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors">
                    "Add User"
                </button>
            </div>

            // Users table
            <div class="bg-slate-800 rounded-xl border border-slate-700 overflow-hidden">
                <table class="w-full">
                    <thead>
                        <tr class="border-b border-slate-700">
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Username"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Display Name"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Role"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Status"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Expires"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Node Access"</th>
                            <th class="px-4 py-3 text-left text-xs font-semibold text-slate-400 uppercase tracking-wider">"Actions"</th>
                        </tr>
                    </thead>
                    <tbody>
                        <tr>
                            <td colspan="7" class="px-4 py-8 text-center text-slate-400 text-sm">
                                "Run 'bilbycast-manager setup' to create the first admin user"
                            </td>
                        </tr>
                    </tbody>
                </table>
            </div>
        </div>
    }
}
