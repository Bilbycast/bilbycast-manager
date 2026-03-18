use leptos::prelude::*;

/// AI API key management page.
#[component]
pub fn AiSettingsPage() -> impl IntoView {
    view! {
        <div>
            <div class="mb-6">
                <h2 class="text-2xl font-bold text-white">"AI API Keys"</h2>
                <p class="text-sm text-slate-400 mt-1">"Configure API keys for AI providers. Keys are encrypted at rest."</p>
            </div>

            <div class="space-y-4">
                <ApiKeyCard
                    provider="OpenAI"
                    description="GPT-4o and other OpenAI models"
                    model_options=vec!["gpt-4o", "gpt-4o-mini", "gpt-4-turbo"]
                />
                <ApiKeyCard
                    provider="Anthropic"
                    description="Claude models for configuration generation"
                    model_options=vec!["claude-sonnet-4-20250514", "claude-haiku-4-5-20251001"]
                />
                <ApiKeyCard
                    provider="Google Gemini"
                    description="Gemini models"
                    model_options=vec!["gemini-2.0-flash", "gemini-1.5-pro"]
                />
            </div>
        </div>
    }
}

#[component]
fn ApiKeyCard(
    provider: &'static str,
    description: &'static str,
    model_options: Vec<&'static str>,
) -> impl IntoView {
    view! {
        <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
            <div class="flex items-center justify-between mb-4">
                <div>
                    <h3 class="text-lg font-semibold text-white">{provider}</h3>
                    <p class="text-sm text-slate-400">{description}</p>
                </div>
                <span class="px-2 py-1 bg-slate-700 text-slate-400 text-xs rounded-full">"Not configured"</span>
            </div>
            <div class="grid grid-cols-2 gap-4">
                <div>
                    <label class="block text-sm font-medium text-slate-300 mb-1">"API Key"</label>
                    <input
                        type="password"
                        placeholder="Enter API key..."
                        class="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white placeholder-slate-400 focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                </div>
                <div>
                    <label class="block text-sm font-medium text-slate-300 mb-1">"Preferred Model"</label>
                    <select class="w-full bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500">
                        {model_options.into_iter().map(|m| view! {
                            <option value=m>{m}</option>
                        }).collect::<Vec<_>>()}
                    </select>
                </div>
            </div>
            <div class="flex justify-end mt-4">
                <button class="px-4 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors">
                    "Save Key"
                </button>
            </div>
        </div>
    }
}
