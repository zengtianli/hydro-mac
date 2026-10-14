import AppKit
import SwiftUI

@main
enum HydroMacMain {
    @MainActor static func main() {
        let words = Array(CommandLine.arguments.dropFirst())
        ProductBasicsTransport.installIfIsolated()
        if words.first == "--help" {
            print(AppLifecycleCLI.help(Lifecycle.command))
            _ = HydroSettingsCLI.run(["preferences", "--help"])
            return
        }
        if HydroSettingsCLI.handles(words.first) { exit(HydroSettingsCLI.run(words)) }
        if AppLifecycleCLI.handles(words.first) { exit(AppLifecycleCLI.run(words, product: Lifecycle.product())) }
        if words.first == "--product-basics-self-test" || words.first == "--ui-self-test" { ProductBasicsSelfTest.runAndExit() }
        HydroMacApp.main()
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
