//! BerryCode Landing Page
//!
//! Leptos CSR mode for landing page

#[cfg(feature = "leptos-ui")]
use leptos::*;

#[cfg(feature = "leptos-ui")]
#[component]
fn App() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gradient-to-br from-purple-900 via-blue-900 to-indigo-900">
            <Header />
            <Hero />
            <Features />
            <CTA />
            <Footer />
        </div>
    }
}

#[cfg(feature = "leptos-ui")]
#[component]
fn Header() -> impl IntoView {
    view! {
        <header class="fixed w-full bg-black/20 backdrop-blur-md z-50">
            <nav class="container mx-auto px-6 py-4 flex justify-between items-center">
                <div class="text-2xl font-bold text-white">
                    "🍓 BerryCode"
                </div>
                <div class="space-x-6">
                    <a href="#features" class="text-white hover:text-purple-300">"Features"</a>
                    <a href="#pricing" class="text-white hover:text-purple-300">"Pricing"</a>
                    <a href="/login" class="bg-purple-600 text-white px-6 py-2 rounded-lg hover:bg-purple-700">
                        "Get Started"
                    </a>
                </div>
            </nav>
        </header>
    }
}

#[cfg(feature = "leptos-ui")]
#[component]
fn Hero() -> impl IntoView {
    view! {
        <section class="pt-32 pb-20 px-6">
            <div class="container mx-auto text-center">
                <h1 class="text-6xl font-bold text-white mb-6">
                    "AI Pair Programming"
                    <br/>
                    <span class="text-transparent bg-clip-text bg-gradient-to-r from-purple-400 to-pink-600">
                        "in Your Terminal"
                    </span>
                </h1>
                <p class="text-xl text-gray-300 mb-8 max-w-2xl mx-auto">
                    "BerryCode brings AI-powered development to your workflow. "
                    "Write code faster, smarter, and better with intelligent assistance."
                </p>
                <div class="flex gap-4 justify-center">
                    <a href="/login" class="bg-gradient-to-r from-purple-600 to-pink-600 text-white px-8 py-4 rounded-lg text-lg font-semibold hover:shadow-xl transform hover:scale-105 transition">
                        "Start Free Trial"
                    </a>
                    <a href="#features" class="border-2 border-white text-white px-8 py-4 rounded-lg text-lg font-semibold hover:bg-white/10 transition">
                        "Learn More"
                    </a>
                </div>
            </div>
        </section>
    }
}

#[cfg(feature = "leptos-ui")]
#[component]
fn Features() -> impl IntoView {
    view! {
        <section id="features" class="py-20 px-6">
            <div class="container mx-auto">
                <h2 class="text-4xl font-bold text-white text-center mb-12">
                    "Powerful Features"
                </h2>
                <div class="grid md:grid-cols-3 gap-8">
                    <FeatureCard
                        icon="🤖"
                        title="AI Assistant"
                        description="Intelligent code completion and suggestions powered by state-of-the-art LLMs"
                    />
                    <FeatureCard
                        icon="🔄"
                        title="Workflow Automation"
                        description="Visual workflow editor for automating development tasks with AI"
                    />
                    <FeatureCard
                        icon="💬"
                        title="Team Collaboration"
                        description="Built-in chat and virtual office for seamless team communication"
                    />
                    <FeatureCard
                        icon="🐳"
                        title="Docker & K8s"
                        description="First-class support for containerized development and deployment"
                    />
                    <FeatureCard
                        icon="🌐"
                        title="Web & Desktop"
                        description="Use in your browser or as a native desktop application"
                    />
                    <FeatureCard
                        icon="🔒"
                        title="Self-Hosted"
                        description="Deploy on your infrastructure. Your code stays private."
                    />
                </div>
            </div>
        </section>
    }
}

#[cfg(feature = "leptos-ui")]
#[component]
fn FeatureCard(
    icon: &'static str,
    title: &'static str,
    description: &'static str,
) -> impl IntoView {
    view! {
        <div class="bg-white/10 backdrop-blur-lg rounded-xl p-6 hover:bg-white/20 transition">
            <div class="text-5xl mb-4">{icon}</div>
            <h3 class="text-2xl font-bold text-white mb-2">{title}</h3>
            <p class="text-gray-300">{description}</p>
        </div>
    }
}

#[cfg(feature = "leptos-ui")]
#[component]
fn CTA() -> impl IntoView {
    view! {
        <section class="py-20 px-6">
            <div class="container mx-auto text-center bg-gradient-to-r from-purple-600 to-pink-600 rounded-2xl p-12">
                <h2 class="text-4xl font-bold text-white mb-4">
                    "Ready to boost your productivity?"
                </h2>
                <p class="text-xl text-white/90 mb-8">
                    "Join thousands of developers using BerryCode"
                </p>
                <a href="/login" class="bg-white text-purple-600 px-8 py-4 rounded-lg text-lg font-semibold hover:shadow-xl transform hover:scale-105 transition inline-block">
                    "Get Started for Free"
                </a>
            </div>
        </section>
    }
}

#[cfg(feature = "leptos-ui")]
#[component]
fn Footer() -> impl IntoView {
    view! {
        <footer class="py-12 px-6 border-t border-white/10">
            <div class="container mx-auto">
                <div class="grid md:grid-cols-4 gap-8">
                    <div>
                        <h4 class="text-white font-bold mb-4">"BerryCode"</h4>
                        <p class="text-gray-400">"AI-powered development platform"</p>
                    </div>
                    <div>
                        <h4 class="text-white font-bold mb-4">"Product"</h4>
                        <ul class="space-y-2 text-gray-400">
                            <li><a href="#features" class="hover:text-white">"Features"</a></li>
                            <li><a href="#pricing" class="hover:text-white">"Pricing"</a></li>
                            <li><a href="/docs" class="hover:text-white">"Documentation"</a></li>
                        </ul>
                    </div>
                    <div>
                        <h4 class="text-white font-bold mb-4">"Company"</h4>
                        <ul class="space-y-2 text-gray-400">
                            <li><a href="/about" class="hover:text-white">"About"</a></li>
                            <li><a href="/blog" class="hover:text-white">"Blog"</a></li>
                            <li><a href="/contact" class="hover:text-white">"Contact"</a></li>
                        </ul>
                    </div>
                    <div>
                        <h4 class="text-white font-bold mb-4">"Connect"</h4>
                        <ul class="space-y-2 text-gray-400">
                            <li><a href="https://github.com/oracleberry/berrcode" class="hover:text-white">"GitHub"</a></li>
                            <li><a href="https://twitter.com" class="hover:text-white">"Twitter"</a></li>
                            <li><a href="https://discord.com" class="hover:text-white">"Discord"</a></li>
                        </ul>
                    </div>
                </div>
                <div class="mt-8 pt-8 border-t border-white/10 text-center text-gray-400">
                    <p>"© 2024 BerryCode. All rights reserved."</p>
                </div>
            </div>
        </footer>
    }
}

#[cfg(feature = "leptos-ui")]
fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}

#[cfg(not(feature = "leptos-ui"))]
fn main() {
    eprintln!("This binary requires the 'leptos-ui' feature to be enabled");
    eprintln!("Build with: cargo build --bin landing_page --features leptos-ui");
    std::process::exit(1);
}
