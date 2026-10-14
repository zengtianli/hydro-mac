import AppKit
import SwiftUI

/// Runs only in a disposable, re-identified copy of the actual product, never the user's App/domain/cloud.
enum ProductBasicsSelfTest {
    @MainActor static func runAndExit() {
        let env = ProcessInfo.processInfo.environment
        guard let out = env["HYDROMAC_BASICS_OUT"], let support = env["APP_LIFECYCLE_SUPPORT_DIR"],
              let cloud = env["APP_LIFECYCLE_CLOUD_DIR"], let domain = Bundle.main.bundleIdentifier,
              domain.hasPrefix("test.hydromac.basics."), !out.isEmpty, !support.isEmpty, !cloud.isEmpty,
              env["AMAP_KEY"] == nil, env["AMAP_API_KEY"] == nil else { fputs("隔离身份/目录缺失，拒绝验收\n", stderr); exit(2) }
        let application = NSApplication.shared
        application.setActivationPolicy(.prohibited)
        let root = NSMenu(title: "Main")
        let appMenu = NSMenu(title: Lifecycle.name), appItem = NSMenuItem(title: Lifecycle.name, action: nil, keyEquivalent: "")
        appItem.submenu = appMenu; root.addItem(appItem)
        for item in AppLifecycleUI.menuItems() { appMenu.addItem(item) }
        let search = NSMenuItem(title: "搜索功能…", action: nil, keyEquivalent: "k"); search.keyEquivalentModifierMask = .command; appMenu.addItem(search)
        application.mainMenu = root
        let configuration = Lifecycle.makeConfiguration()!
        AppLifecycleUI.install(name: Lifecycle.name, configuration: configuration, updateSource: Lifecycle.updateSource, startConfiguration: false)
        let settings = HydroSettings.shared; settings.install(configuration: configuration)
        var checks: [String: Bool] = [:], cases: [[String: Any]] = [], notes: [String: String] = [:], shots: [String] = []
        let output = URL(fileURLWithPath: out); try! FileManager.default.createDirectory(at: output, withIntermediateDirectories: true)
        struct Reply { let code: Int32; let body: [String: Any]; let stdout: String }
        func pump(_ seconds: Double) { RunLoop.main.run(until: Date().addingTimeInterval(seconds)) }
        func call(_ words: [String], cloudOverride: String? = nil) -> Reply {
            let process = Process(), pipe = Pipe(), err = Pipe()
            process.executableURL = Bundle.main.executableURL!
            process.arguments = words + ["--json"]; var child = env
            if let cloudOverride { child["APP_LIFECYCLE_CLOUD_DIR"] = cloudOverride }
            process.environment = child; process.standardOutput = pipe; process.standardError = err; process.standardInput = FileHandle.nullDevice
            do { try process.run() } catch { return Reply(code: -1, body: [:], stdout: error.localizedDescription) }
            let deadline = Date().addingTimeInterval(40)
            while process.isRunning && Date() < deadline { pump(0.005) }
            if process.isRunning { process.terminate(); process.waitUntilExit() }
            let text = String(decoding: pipe.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
            let stderr = String(decoding: err.fileHandleForReading.readDataToEndOfFile(), as: UTF8.self)
            let body = ((try? JSONSerialization.jsonObject(with: Data(text.utf8))) as? [String: Any]) ?? [:]
            cases.append(["argv": words, "exit": process.terminationStatus, "response": body, "stderr": stderr])
            return Reply(code: process.terminationStatus, body: body, stdout: text)
        }
        func export() -> Data {
            let file = output.appendingPathComponent("portable.json")
            let result = call(["config", "export", "-o", file.path, "--force"])
            return result.code == 0 ? (try? Data(contentsOf: file)) ?? Data() : Data()
        }
        func write(_ object: [String: Any], to file: URL) throws { try JSONSerialization.data(withJSONObject: object, options: [.sortedKeys]).write(to: file, options: .atomic) }
        func cycle(cloudOverride: String? = nil) -> Reply {
            _ = call(["config", "sync", "off", "--yes"])
            return call(["config", "sync", "on", "--yes"], cloudOverride: cloudOverride)
        }
        do {
            for appearance in [NSAppearance.Name.aqua, .darkAqua] {
                let filename = appearance == .aqua ? "settings-light.png" : "settings-dark.png"
                let result = try AppLifecycleUI.shared.offscreenSnapshot(to: output.appendingPathComponent(filename), appearance: appearance)
                for (key, value) in result { checks["settings.\(filename).\(key)"] = value }
                shots.append(filename)
            }
            settings.calculatorPopup.selectItem(at: 1)
            let acted = application.sendAction(settings.calculatorPopup.action!, to: settings.calculatorPopup.target, from: settings.calculatorPopup)
            checks["settings.interaction"] = acted && HydroPreferences.calculator == "capacity" && settings.calculatorPopup.titleOfSelectedItem == CALCULATORS[1].name
            let persisted = call(["preferences", "status"])
            checks["settings.persisted_readback"] = persisted.code == 0 && persisted.body["startup_calculator"] as? String == "capacity"
            // The production migration adapter must reject an invalid calculator before changing any values.
            let beforeBad = export(); let bad = output.appendingPathComponent("bad-import.json")
            try write(["version": 1, "product": Lifecycle.productID, "values": ["defaults." + HydroPreferences.calculatorKey: "missing-calculator"]], to: bad)
            let rejected = call(["config", "import", bad.path, "--yes"])
            checks["settings.error_preserves_data"] = rejected.code == 1 && !beforeBad.isEmpty && beforeBad == export()
            UserDefaults.standard.set("isolated-test-credential", forKey: "geocode.amapKey")
            checks["settings.credentials_excluded"] = !String(decoding: export(), as: UTF8.self).contains("isolated-test-credential")

            settings.attachMenu(); HydroNavigation.shared.selection = CALCULATORS.last
            let item = settings.shortcuts.menuItem!
            let didAction = application.sendAction(item.action!, to: item.target, from: item)
            let host = NSHostingView(rootView: ContentView(navigation: HydroNavigation.shared).frame(width: 1180, height: 760))
            let window = NSWindow(contentRect: NSRect(x: -20000, y: -20000, width: 1180, height: 760), styleMask: [.titled], backing: .buffered, defer: false)
            window.isReleasedWhenClosed = false; window.contentView = host; host.layoutSubtreeIfNeeded()
            checks["shortcuts.action_path"] = didAction && item.keyEquivalent == "r" && item.keyEquivalentModifierMask == .command && HydroNavigation.shared.selection?.key == "capacity"
            settings.shortcutToggle.state = .off
            _ = application.sendAction(settings.shortcutToggle.action!, to: settings.shortcutToggle.target, from: settings.shortcutToggle)
            let disabled = call(["shortcuts", "status"])
            let wasDisabled = disabled.code == 0 && disabled.body["enabled"] as? Bool == false && item.keyEquivalent.isEmpty
            settings.shortcutToggle.state = .on
            _ = application.sendAction(settings.shortcutToggle.action!, to: settings.shortcutToggle.target, from: settings.shortcutToggle)
            let restored = call(["shortcuts", "status"])
            checks["shortcuts.disable_restore"] = wasDisabled && restored.code == 0 && restored.body["enabled"] as? Bool == true && item.keyEquivalent == "r"
            settings.keyPopup.selectItem(withTitle: "Q"); settings.modifiersPopup.selectItem(at: 0)
            _ = application.sendAction(settings.keyPopup.action!, to: settings.keyPopup.target, from: settings.keyPopup)
            let reservedRejected = !settings.errorLabel.stringValue.isEmpty && item.keyEquivalent == "r"
            settings.keyPopup.selectItem(withTitle: "K")
            _ = application.sendAction(settings.keyPopup.action!, to: settings.keyPopup.target, from: settings.keyPopup)
            let bindingBefore = export(); let reservedCLI = call(["shortcuts", "set", "--key", "q", "--yes"])
            checks["shortcuts.conflict_or_reserved_keys"] = reservedRejected && !settings.errorLabel.stringValue.isEmpty && item.keyEquivalent == "r" && reservedCLI.code == 2 && export() == bindingBefore
            settings.keyPopup.selectItem(withTitle: "T"); settings.modifiersPopup.selectItem(at: 1)
            _ = application.sendAction(settings.keyPopup.action!, to: settings.keyPopup.target, from: settings.keyPopup)
            let changed = call(["shortcuts", "status"])
            checks["shortcuts.saved_binding_readback"] = changed.code == 0 && (changed.body["binding"] as? [String: Any])?["key"] as? String == "t" && item.keyEquivalent == "t"
            let cliBinding = call(["shortcuts", "set", "--key", "t", "--modifiers", "command,shift", "--yes"])
            checks["shortcuts.cli_binding_uses_shared_adapter"] = cliBinding.code == 0 && (cliBinding.body["binding"] as? [String: Any])?["key"] as? String == "t"

            let cloudFile = URL(fileURLWithPath: cloud).appendingPathComponent(Lifecycle.productID + ".json")
            let pushed = cycle()
            var envelope = ((try? JSONSerialization.jsonObject(with: Data(contentsOf: cloudFile))) as? [String: Any]) ?? [:]
            var values = envelope["values"] as? [String: Any] ?? [:]
            let actuallyPushed = pushed.code == 0 && values["defaults." + HydroPreferences.calculatorKey] as? String == "capacity"
            values["defaults." + HydroPreferences.calculatorKey] = "reservoir"; envelope["values"] = values; try write(envelope, to: cloudFile)
            let pulled = cycle(); let remoteRead = call(["preferences", "status"])
            checks["icloud.push_pull"] = actuallyPushed && pulled.code == 0 && remoteRead.body["startup_calculator"] as? String == "reservoir"
            let good = try Data(contentsOf: cloudFile); let localBefore = export()
            try Data("{broken".utf8).write(to: cloudFile, options: .atomic)
            let corrupt = cycle(); let preserved = export() == localBefore
            try good.write(to: cloudFile, options: .atomic)
            let recovered = cycle()
            _ = call(["preferences", "set", "--calculator", "irrigation", "--yes"])
            envelope = (try JSONSerialization.jsonObject(with: Data(contentsOf: cloudFile))) as! [String: Any]
            values = envelope["values"] as! [String: Any]
            values["defaults." + HydroPreferences.calculatorKey] = "district"; values["defaults." + AppShortcuts.enabledKey] = false
            envelope["values"] = values; try write(envelope, to: cloudFile)
            let conflict = cycle(); let merged = call(["preferences", "status"])
            checks["icloud.conflict_or_corrupt_recovery"] = corrupt.code == 1 && preserved && recovered.code == 0 && conflict.code == 0 && merged.body["startup_calculator"] as? String == "irrigation" && merged.body["enabled"] as? Bool == false
            let beforeUnavailable = export(); let inaccessible = output.appendingPathComponent("unavailable-cloud")
            try Data("not a directory".utf8).write(to: inaccessible)
            let unavailable = cycle(cloudOverride: inaccessible.path); let local = call(["preferences", "status"])
            checks["icloud.unavailable_local_fallback"] = unavailable.code == 1 && !beforeUnavailable.isEmpty && export() == beforeUnavailable && local.body["startup_calculator"] as? String == "irrigation"
            let cloudBeforeOff = try Data(contentsOf: cloudFile)
            let off = call(["config", "sync", "off", "--yes"])
            let cloudAfterOff = try Data(contentsOf: cloudFile)
            checks["icloud.disable_preserves_data"] = off.code == 0 && off.body["sync_enabled"] as? Bool == false && export() == beforeUnavailable && cloudAfterOff == cloudBeforeOff

            let optionsBefore = call(["update", "options"])
            let savedOptions = call(["update", "options", "--auto-check", "on", "--interval", "6", "--yes"])
            let freshOptions = call(["update", "options"])
            let invalidOptions = call(["update", "options", "--interval", "0", "--yes"])
            let unchangedOptions = call(["update", "options"])
            checks["updates.options_or_store_entry"] = (optionsBefore.body["options"] as? [String: Any])?["automaticCheck"] as? Bool == false && savedOptions.code == 0 && freshOptions.code == 0 && (freshOptions.body["options"] as? [String: Any])?["intervalHours"] as? Int == 6 && (freshOptions.body["options"] as? [String: Any])?["automaticCheck"] as? Bool == true && invalidOptions.code == 2 && NSDictionary(dictionary: freshOptions.body["options"] as? [String: Any] ?? [:]).isEqual(to: unchangedOptions.body["options"] as? [String: Any] ?? [:])
            let feed = URL(fileURLWithPath: support).appendingPathComponent("github-release.json")
            try FileManager.default.createDirectory(at: feed.deletingLastPathComponent(), withIntermediateDirectories: true)
            let beforeUpdates = export(); let missing = call(["update", "check"])
            try Data("{broken".utf8).write(to: feed, options: .atomic); let broken = call(["update", "check"])
            let validFeed = try Data(contentsOf: URL(fileURLWithPath: support).appendingPathComponent("good-github-release.json"))
            try validFeed.write(to: feed, options: .atomic)
            let update = call(["update", "check"]); let upgrade = update.body["upgrade"] as? [String: Any] ?? [:]
            AppLifecycleUI.shared.checkForUpdates(present: false)
            let deadline = Date().addingTimeInterval(10)
            func labels(_ view: NSView) -> [String] { (view as? NSTextField).map { [$0.stringValue] } ?? view.subviews.flatMap(labels) }
            while !application.windows.compactMap(\.contentView).flatMap(labels).contains(where: { $0.contains("有新版") }) && Date() < deadline { pump(0.02) }
            checks["updates.check_result"] = update.code == 0 && update.body["state"] as? String == "update_available" && upgrade["download_url"] as? String == "https://github.com/zengtianli/hydro-mac/releases/download/v99.0.0/HydroMac-arm64.zip" && upgrade["button"] as? String == "下载新版…" && application.windows.compactMap(\.contentView).flatMap(labels).contains { $0.contains("有新版") }
            checks["updates.failure_recovery"] = missing.code == 1 && broken.code == 1 && update.code == 0
            let dry = call(["update", "install", "--dry-run"])
            checks["updates.data_preservation"] = dry.code == 1 && (dry.body["error"] as? [String: Any])?["code"] as? String == "manual_install" && export() == beforeUpdates && UserDefaults.standard.string(forKey: "geocode.amapKey") == "isolated-test-credential"
            for appearance in [NSAppearance.Name.aqua, .darkAqua] {
                let filename = appearance == .aqua ? "updates-light.png" : "updates-dark.png"
                let result = try AppLifecycleUI.shared.offscreenSnapshot(to: output.appendingPathComponent(filename), appearance: appearance, showingUpdateOptions: true)
                for (key, value) in result { checks["updates.\(filename).\(key)"] = value }; shots.append(filename)
            }
            checks["settings.never_visible_or_key"] = application.activationPolicy() == .prohibited && application.windows.allSatisfy { !$0.isVisible && !$0.isKeyWindow }
            window.close()
        } catch { notes["error"] = error.localizedDescription; checks["settings.execution_completed"] = false }
        let report: [String: Any] = ["checks": checks, "cases": cases, "notes": notes, "screenshots": shots,
            "scope": "Actual product in a re-identified copy; native settings/action paths offscreen, fresh CLI processes, isolated directory sync and release feed; no Apple cross-device transmission."]
        let data = try! JSONSerialization.data(withJSONObject: report, options: [.sortedKeys, .prettyPrinted])
        try! data.write(to: output.appendingPathComponent("native.json"), options: .atomic)
        print(String(decoding: data, as: UTF8.self)); exit(!checks.isEmpty && checks.values.allSatisfy { $0 } ? 0 : 1)
    }
}
