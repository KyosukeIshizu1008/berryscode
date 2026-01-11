//! BerryCode Landing Page - Tauri Desktop Version
//!
//! Leptos + Tauri v2 for native desktop experience

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
use leptos::*;

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
#[component]
fn App() -> impl IntoView {
    view! {
        <div class="min-h-screen bg-gradient-to-br from-purple-900 via-blue-900 to-indigo-900">
            <DesktopHeader />
            <Hero />
            <Features />
            <DownloadCTA />
            <Footer />
        </div>
    }
}

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
#[component]
fn DesktopHeader() -> impl IntoView {
    view! {
        <header class="fixed w-full bg-black/20 backdrop-blur-md z-50" data-tauri-drag-region>
            <nav class="px-6 py-4 flex justify-between items-center">
                <div class="text-2xl font-bold text-white">
                    "🍓 BerryCode Desktop"
                </div>
                <div class="space-x-6">
                    <a href="#features" class="text-white hover:text-purple-300">"Features"</a>
                    <button
                        on:click=|_| {
                            #[cfg(feature = "tauri")]
                            {
                                use tauri::Manager;
                                let window = tauri::Window::get_current_window().unwrap();
                                window.close().unwrap();
                            }
                        }
                        class="bg-purple-600 text-white px-6 py-2 rounded-lg hover:bg-purple-700"
                    >
                        "Start Using"
                    </button>
                </div>
            </nav>
        </header>
    }
}

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
#[component]
fn Hero() -> impl IntoView {
    view! {
        <section class="pt-32 pb-20 px-6">
            <div class="container mx-auto text-center">
                <h1 class="text-6xl font-bold text-white mb-6">
                    "AI Pair Programming"
                    <br/>
                    <span class="text-transparent bg-clip-text bg-gradient-to-r from-purple-400 to-pink-600">
                        "Native Desktop Experience"
                    </span>
                </h1>
                <p class="text-xl text-gray-300 mb-8 max-w-2xl mx-auto">
                    "Fast, secure, and offline-capable. BerryCode Desktop gives you "
                    "the full power of AI coding assistance without compromising performance."
                </p>
                <div class="flex gap-4 justify-center">
                    <button
                        on:click=|_| {
                            // Tauriでメインアプリウィンドウを開く
                            #[cfg(feature = "tauri")]
                            {
                                use tauri::Manager;
                                let app = tauri::AppHandle::get_current();
                                app.emit_all("open-main-app", ()).unwrap();
                            }
                        }
                        class="bg-gradient-to-r from-purple-600 to-pink-600 text-white px-8 py-4 rounded-lg text-lg font-semibold hover:shadow-xl transform hover:scale-105 transition"
                    >
                        "Launch BerryCode"
                    </button>
                </div>
            </div>
        </section>
    }
}

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
#[component]
fn Features() -> impl IntoView {
    view! {
        <section id="features" class="py-20 px-6">
            <div class="container mx-auto">
                <h2 class="text-4xl font-bold text-white text-center mb-12">
                    "Desktop Advantages"
                </h2>
                <div class="grid md:grid-cols-3 gap-8">
                    <FeatureCard
                        icon="⚡"
                        title="Native Performance"
                        description="Built with Rust and Tauri for maximum speed and efficiency"
                    />
                    <FeatureCard
                        icon="🔒"
                        title="Secure"
                        description="Your code never leaves your machine. Complete privacy guaranteed."
                    />
                    <FeatureCard
                        icon="📴"
                        title="Offline Ready"
                        description="Work without internet. AI models can run locally."
                    />
                    <FeatureCard
                        icon="💾"
                        title="Low Footprint"
                        description="Only ~50MB installed. Smaller than Electron apps."
                    />
                    <FeatureCard
                        icon="🎨"
                        title="Native UI"
                        description="Uses your system's native rendering for a familiar feel"
                    />
                    <FeatureCard
                        icon="🔄"
                        title="Auto Updates"
                        description="Seamless updates in the background. Always stay current."
                    />
                </div>
            </div>
        </section>
    }
}

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
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

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
#[component]
fn DownloadCTA() -> impl IntoView {
    let (os_name, download_text) = get_platform_info();

    view! {
        <section class="py-20 px-6">
            <div class="container mx-auto text-center bg-gradient-to-r from-purple-600 to-pink-600 rounded-2xl p-12">
                <h2 class="text-4xl font-bold text-white mb-4">
                    "Download for " {os_name}
                </h2>
                <p class="text-xl text-white/90 mb-8">
                    {download_text}
                </p>
                <button
                    on:click=|_| {
                        #[cfg(feature = "tauri")]
                        {
                            use tauri::api::shell;
                            shell::open(
                                &shell::Scope::Global,
                                "https://github.com/oracleberry/berrcode/releases",
                                None,
                            ).unwrap();
                        }
                    }
                    class="bg-white text-purple-600 px-8 py-4 rounded-lg text-lg font-semibold hover:shadow-xl transform hover:scale-105 transition inline-block"
                >
                    "Download Now"
                </button>
            </div>
        </section>
    }
}

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
fn get_platform_info() -> (&'static str, &'static str) {
    #[cfg(target_os = "macos")]
    return ("macOS", "Universal binary for Intel and Apple Silicon");

    #[cfg(target_os = "windows")]
    return ("Windows", "Compatible with Windows 10 and 11");

    #[cfg(target_os = "linux")]
    return ("Linux", "Available as AppImage, .deb, and .rpm");

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return ("Your Platform", "Download available for multiple platforms");
}

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
#[component]
fn Footer() -> impl IntoView {
    view! {
        <footer class="py-12 px-6 border-t border-white/10">
            <div class="container mx-auto text-center">
                <p class="text-gray-400 mb-4">
                    "BerryCode Desktop v" {env!("CARGO_PKG_VERSION")}
                </p>
                <p class="text-gray-400">
                    "Built with ❤️ using Rust, Leptos, and Tauri v2"
                </p>
            </div>
        </footer>
    }
}

#[cfg(all(feature = "leptos-ui", feature = "tauri"))]
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![])
        .setup(|app| {
            let window = app.get_window("main").unwrap();

            // Set up window
            window.set_title("BerryCode - Welcome").unwrap();
            window.set_size(tauri::Size::Physical(tauri::PhysicalSize {
                width: 1200,
                height: 800,
            })).unwrap();

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(not(all(feature = "leptos-ui", feature = "tauri")))]
fn main() {
    eprintln!("This binary requires both 'leptos-ui' and 'tauri' features");
    eprintln!("Build with: cargo build --bin landing_page_desktop --features leptos-ui,tauri");
    std::process::exit(1);
}
