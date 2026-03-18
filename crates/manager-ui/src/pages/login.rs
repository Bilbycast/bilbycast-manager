use leptos::prelude::*;

/// Login page with username/password form.
#[component]
pub fn LoginPage() -> impl IntoView {
    view! {
        <div class="w-full max-w-md">
            <div class="bg-slate-800 rounded-xl shadow-2xl border border-slate-700 p-8">
                // Logo
                <div class="text-center mb-8">
                    <h1 class="text-3xl font-bold text-blue-400">"bilbycast"</h1>
                    <p class="text-sm text-slate-400 mt-1">"Edge Node Manager"</p>
                </div>

                // Login form
                <form class="space-y-6">
                    <div>
                        <label class="block text-sm font-medium text-slate-300 mb-1">"Username"</label>
                        <input
                            type="text"
                            name="username"
                            required=true
                            class="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-2.5 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            placeholder="Enter username"
                        />
                    </div>

                    <div>
                        <label class="block text-sm font-medium text-slate-300 mb-1">"Password"</label>
                        <input
                            type="password"
                            name="password"
                            required=true
                            class="w-full bg-slate-700 border border-slate-600 rounded-lg px-4 py-2.5 text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                            placeholder="Enter password"
                        />
                    </div>

                    <button
                        type="submit"
                        class="w-full bg-blue-600 hover:bg-blue-700 text-white font-medium py-2.5 rounded-lg transition-colors focus:outline-none focus:ring-2 focus:ring-blue-500 focus:ring-offset-2 focus:ring-offset-slate-800"
                    >
                        "Sign In"
                    </button>
                </form>
            </div>
        </div>
    }
}
