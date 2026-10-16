import AppKit
import SwiftUI

@main
enum HydroMacMain {
    @MainActor static func main() async {
        let words = Array(CommandLine.arguments.dropFirst())
        ProductBasicsTransport.installIfIsolated()
        if words.first == "--help" {
            print(HydroCommandCLI.help)
            print(AppLifecycleCLI.help(Lifecycle.command))
            _ = HydroSettingsCLI.run(["preferences", "--help"])
            return
        }
        if HydroSettingsCLI.handles(words.first) { exit(HydroSettingsCLI.run(words)) }
        if HydroCommandCLI.handles(words.first) { exit(await HydroCommandCLI.run(words)) }
        if AppLifecycleCLI.handles(words.first) { exit(AppLifecycleCLI.run(words, product: Lifecycle.product())) }
        if words.first == "--product-basics-self-test" { ProductBasicsSelfTest.runAndExit() }
        if words.first == "--ui-self-test" { UISelfTest.runAndExit(outDir: ProcessInfo.processInfo.environment["HYDROMAC_UI_OUT"] ?? "") }
        if LaneSignal.quiet || words.first == "--background" { quietMain(); return }
        if let word = words.first, !word.hasPrefix("-") { fputs("未知命令；用 hydromac --help 查看。\n", stderr); exit(2) }
        HydroMacApp.main()
    }

    @MainActor private static func quietMain() {
        let app = NSApplication.shared; app.setActivationPolicy(.prohibited)
        let window = NSWindow(contentRect: NSRect(x: -20000, y: -20000, width: 1180, height: 760), styleMask: [.titled], backing: .buffered, defer: false)
        window.isReleasedWhenClosed = false
        let host = NSHostingView(rootView: ContentView(navigation: HydroNavigation(readPreferences: false)).frame(width: 1180, height: 760))
        window.contentView = host
        Task { @MainActor in host.layoutSubtreeIfNeeded(); if let rep = host.bitmapImageRepForCachingDisplay(in: host.bounds) { host.cacheDisplay(in: host.bounds, to: rep); LaneSignal.ready("main") } }
        app.run(); withExtendedLifetime(window) {}
    }
}

/// 公开版原生壳；配置/快捷键/同步/更新均调用总部生命周期实现。
struct HydroMacApp: App {
    @NSApplicationDelegateAdaptor(LaneAppDelegate.self) private var laneDelegate
    init() { Lifecycle.installApp() }
    var body: some Scene {
        WindowGroup {
            ContentView()
                .frame(minWidth: 900, minHeight: 600)
                .onAppear { LaneSignal.ready("window"); LaneSignal.applyWindowFrame() }
        }
        .windowResizability(.contentSize)
        .defaultSize(width: 1180, height: 760)
        .commands { PaletteCommands() }
    }
}
