use leptos::prelude::*;

/// AI assistant page for configuration generation and system queries.
#[component]
pub fn AiAssistantPage() -> impl IntoView {
    view! {
        <div>
            <div class="mb-6">
                <h2 class="text-2xl font-bold text-white">"AI Configuration Assistant"</h2>
                <p class="text-sm text-slate-400 mt-1">"Use AI to generate flow configurations, analyze anomalies, and query system status"</p>
            </div>

            <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
                // Config generation
                <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
                    <h3 class="text-lg font-semibold text-white mb-4">"Generate Flow Configuration"</h3>
                    <textarea
                        placeholder="Describe the flow you want to create. For example: 'Create an SRT listener on port 9000 with AES-256 encryption that sends to RTP multicast 239.1.1.1:5004 with 10x10 FEC'"
                        class="w-full h-32 bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-sm text-white placeholder-slate-400 resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                    <div class="flex items-center justify-between mt-4">
                        <select class="bg-slate-700 border border-slate-600 rounded-lg px-3 py-2 text-sm text-white focus:outline-none focus:ring-2 focus:ring-blue-500">
                            <option value="">"Select AI Provider"</option>
                            <option value="openai">"OpenAI (GPT)"</option>
                            <option value="anthropic">"Anthropic (Claude)"</option>
                            <option value="gemini">"Google (Gemini)"</option>
                        </select>
                        <button class="px-6 py-2 bg-purple-600 hover:bg-purple-700 text-white text-sm font-medium rounded-lg transition-colors">
                            "Generate"
                        </button>
                    </div>
                </div>

                // Anomaly analysis
                <div class="bg-slate-800 rounded-xl border border-slate-700 p-6">
                    <h3 class="text-lg font-semibold text-white mb-4">"Analyze System"</h3>
                    <textarea
                        placeholder="Ask about your system. For example: 'Which nodes have high packet loss?' or 'Analyze recent alarms on Node 1'"
                        class="w-full h-32 bg-slate-700 border border-slate-600 rounded-lg px-4 py-3 text-sm text-white placeholder-slate-400 resize-none focus:outline-none focus:ring-2 focus:ring-blue-500"
                    />
                    <div class="flex justify-end mt-4">
                        <button class="px-6 py-2 bg-blue-600 hover:bg-blue-700 text-white text-sm font-medium rounded-lg transition-colors">
                            "Analyze"
                        </button>
                    </div>
                </div>
            </div>

            // Results area
            <div class="mt-6 bg-slate-800 rounded-xl border border-slate-700 p-6 min-h-[200px]">
                <h3 class="text-lg font-semibold text-white mb-4">"Results"</h3>
                <p class="text-slate-400 text-sm">"AI results will appear here. Configure your API keys in AI Settings first."</p>
            </div>
        </div>
    }
}
