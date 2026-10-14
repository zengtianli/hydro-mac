import AppKit
import SwiftUI
import CryptoKit

/// Product-owned adapter; storage, migration, shortcuts and updates use the shared engines.
enum HydroPreferences {
    static let calculatorKey = "hydromac.startupCalculator"
    static var defaults: UserDefaults {
        let env = ProcessInfo.processInfo.environment
        if let support = env["APP_LIFECYCLE_SUPPORT_DIR"], Bundle.main.bundleIdentifier == Lifecycle.productID {
            let suffix = SHA256.hash(data: Data(support.utf8)).prefix(8).map { String(format: "%02x", $0) }.joined()
            return UserDefaults(suiteName: "test.hydromac." + suffix)!
        }
        return .standard
    }
    static var calculator: String { defaults.string(forKey: calculatorKey).flatMap { value in CALCULATORS.contains { $0.key == value } ? value : nil } ?? "annual" }
    static func validate(_ key: String, _ value: Any) throws {
        switch key {
        case calculatorKey:
            guard let value = value as? String, CALCULATORS.contains(where: { $0.key == value }) else { throw AppUpdateError("请选择已有的计算器。") }
        case AppShortcuts.enabledKey:
            guard let n = value as? NSNumber, CFGetTypeID(n) == CFBooleanGetTypeID() else { throw AppUpdateError("快捷键开关必须是布尔值。") }
        case AppShortcuts.bindingKey:
            guard let d = value as? [String: Any], Set(d.keys) == ["key", "modifiers"], let key = d["key"] as? String,
                  let modifiers = d["modifiers"] as? NSNumber else { throw AppUpdateError("快捷键配置格式错误。") }
            try AppShortcuts.validate(.init(key: key, modifiers: .init(rawValue: modifiers.uintValue)))
            if key.lowercased() == "k" && modifiers.uintValue == NSEvent.ModifierFlags.command.rawValue { throw AppUpdateError("⌘K 已用于搜索功能，请选择其他组合。") }
        case "appUpdateOptions":
            let data = try JSONSerialization.data(withJSONObject: value)
            try AppUpdatePreferences.validate(JSONDecoder().decode(AppUpdateOptions.self, from: data))
        default: throw AppUpdateError("此字段不允许迁移。")
        }
    }
    static func setCalculator(_ value: String) throws {
        try validate(calculatorKey, value)
        defaults.set(value, forKey: calculatorKey); defaults.synchronize()
    }
}

/// The same observable navigation state is consumed by ContentView and the real shortcut action.
final class HydroNavigation: ObservableObject {
    static let shared = HydroNavigation()
    @Published var selection: Calculator?
    init(readPreferences: Bool = true) {
        selection = CALCULATORS.first { $0.key == (readPreferences ? HydroPreferences.calculator : "annual") }
    }
    func openDefaultCalculator() { selection = CALCULATORS.first { $0.key == HydroPreferences.calculator } }
}

final class HydroSettings: NSObject {
    static let shared = HydroSettings()
    let calculatorPopup = NSPopUpButton()
    let shortcutToggle = NSSwitch()
    let keyPopup = NSPopUpButton()
    let modifiersPopup = NSPopUpButton()
    let shortcutStatus = NSTextField(labelWithString: "")
    let errorLabel = NSTextField(wrappingLabelWithString: "")
    lazy var shortcuts = AppShortcuts(defaults: HydroPreferences.defaults) { HydroNavigation.shared.openDefaultCalculator() }
    private var configuration: AppConfiguration?
    private var errorRow: NSView?
    private var observers: [NSObjectProtocol] = []
    private let modifiers: [NSEvent.ModifierFlags] = [.command, [.command, .shift], [.command, .option], [.command, .control]]

    func install(configuration: AppConfiguration) {
        self.configuration = configuration
        calculatorPopup.removeAllItems(); calculatorPopup.addItems(withTitles: CALCULATORS.map(\.name))
        keyPopup.removeAllItems(); keyPopup.addItems(withTitles: Array("abcdefghijklmnopqrstuvwxyz0123456789").map { String($0).uppercased() })
        modifiersPopup.removeAllItems(); modifiersPopup.addItems(withTitles: ["⌘", "⇧⌘", "⌥⌘", "⌃⌘"])
        calculatorPopup.target = self; calculatorPopup.action = #selector(changeCalculator(_:))
        shortcutToggle.target = self; shortcutToggle.action = #selector(changeEnabled(_:))
        for popup in [keyPopup, modifiersPopup] { popup.target = self; popup.action = #selector(changeBinding(_:)) }
        errorLabel.textColor = .systemRed
        AppLifecycleUI.shared.productGroups = { [weak self] in self?.groups() ?? [] }
        AppLifecycleUI.shared.willShow = { [weak self] in self?.refresh() }
        if observers.isEmpty {
            for name in [NSApplication.didFinishLaunchingNotification, NSApplication.didBecomeActiveNotification] {
                observers.append(NotificationCenter.default.addObserver(forName: name, object: nil, queue: .main) { [weak self] _ in self?.attachMenu() })
            }
            observers.append(DistributedNotificationCenter.default().addObserver(forName: Self.changedName, object: nil, queue: .main) { [weak self] _ in self?.refresh() })
        }
        refresh(); attachMenu()
    }
    static var changedName: Notification.Name { .init("io.github.zengtianli.hydromac.preferences." + (Bundle.main.bundleIdentifier ?? Lifecycle.productID)) }
    static func notifyChanged() { DistributedNotificationCenter.default().postNotificationName(changedName, object: nil, userInfo: nil, deliverImmediately: true) }
    func attachMenu() { shortcuts.installMenu(); shortcuts.menuItem?.title = "打开默认计算器"; shortcuts.refresh() }
    func refresh() {
        HydroPreferences.defaults.synchronize()
        calculatorPopup.selectItem(at: CALCULATORS.firstIndex { $0.key == HydroPreferences.calculator } ?? 0)
        shortcutToggle.state = shortcuts.enabled ? .on : .off
        keyPopup.selectItem(withTitle: shortcuts.binding.key.uppercased())
        modifiersPopup.selectItem(at: modifiers.firstIndex(of: shortcuts.binding.modifiers) ?? 0)
        shortcutStatus.stringValue = shortcuts.descriptionText
        errorRow?.isHidden = errorLabel.stringValue.isEmpty
        shortcuts.refresh()
    }
    private func groups() -> [NSView] {
        let errorRow = AppLifecycleUI.row("", detail: errorLabel)
        self.errorRow = errorRow; errorRow.isHidden = errorLabel.stringValue.isEmpty
        return [AppLifecycleUI.group("使用偏好", rows: [
            AppLifecycleUI.row("启动时打开", detail: AppLifecycleUI.detailLabel("启动与“打开默认计算器”使用同一选择，不包含输入文件或计算结果。"), accessory: calculatorPopup)
        ]), AppLifecycleUI.group("应用快捷键", rows: [
            AppLifecycleUI.row("打开默认计算器", detail: shortcutStatus, accessory: shortcutToggle),
            AppLifecycleUI.row("组合键", detail: AppLifecycleUI.detailLabel("只在 HydroMac 内生效；⌘K 用于搜索，系统编辑键和已有菜单键不可占用。"), accessory: NSStackView(views: [modifiersPopup, keyPopup])),
            errorRow
        ])]
    }
    @objc func changeCalculator(_ sender: NSPopUpButton) {
        do { try HydroPreferences.setCalculator(CALCULATORS[sender.indexOfSelectedItem].key); errorLabel.stringValue = ""; configuration?.reconcile(); Self.notifyChanged() }
        catch { errorLabel.stringValue = error.localizedDescription }
        refresh()
    }
    @objc func changeEnabled(_ sender: NSSwitch) { shortcuts.setEnabled(sender.state == .on); configuration?.reconcile(); Self.notifyChanged(); refresh() }
    @objc func changeBinding(_ sender: NSPopUpButton) {
        do {
            let binding = AppShortcuts.Binding(key: (keyPopup.titleOfSelectedItem ?? "R").lowercased(), modifiers: modifiers[max(0, modifiersPopup.indexOfSelectedItem)])
            try HydroPreferences.validate(AppShortcuts.bindingKey, binding.dictionary); try shortcuts.setBinding(binding)
            errorLabel.stringValue = ""; configuration?.reconcile(); Self.notifyChanged()
        } catch { errorLabel.stringValue = error.localizedDescription }
        refresh()
    }
}

/// Product-specific preference commands only; config/sync/update stay in the shared command layer.
enum HydroSettingsCLI {
    static func handles(_ verb: String?) -> Bool { verb == "preferences" || verb == "shortcuts" }
    static func run(_ arguments: [String]) -> Int32 {
        let words = arguments.filter { $0 != "--json" }; let json = arguments.contains("--json")
        let verb = words.first ?? "", sub = words.dropFirst().first ?? ""
        var body: [String: Any] = ["ok": true, "command": "\(verb) \(sub)"]
        var code: Int32 = 0
        do {
            if words.contains("--help") || sub == "help" {
                print("usage: hydromac preferences status | set --calculator <key> --yes; hydromac shortcuts status | set --key <letter> [--modifiers command,shift|command,option|command,control] --yes | enable|disable --yes (all support --json)")
                return 0
            }
            let shortcuts = AppShortcuts(defaults: HydroPreferences.defaults, action: {})
            if sub == "status", words.count == 2 { }
            else {
                guard words.last == "--yes" else { throw AppUpdateError("保存必须显式加 --yes。") }
                if verb == "preferences", sub == "set", words.count == 5, words[2] == "--calculator" {
                    try HydroPreferences.setCalculator(words[3])
                } else if verb == "shortcuts", (sub == "enable" || sub == "disable"), words.count == 3 {
                    shortcuts.setEnabled(sub == "enable")
                } else if verb == "shortcuts", sub == "set", [5, 7].contains(words.count), words[2] == "--key" {
                    var flags: NSEvent.ModifierFlags = .command
                    if words.count == 7 {
                        guard words[4] == "--modifiers" else { throw AppUpdateError("未知参数。") }
                        flags = []
                        let choices: [String: NSEvent.ModifierFlags] = ["command": .command, "shift": .shift, "option": .option, "control": .control]
                        for word in words[5].split(separator: ",") { guard let flag = choices[String(word)] else { throw AppUpdateError("未知修饰键。") }; flags.insert(flag) }
                    }
                    let binding = AppShortcuts.Binding(key: words[3].lowercased(), modifiers: flags)
                    try HydroPreferences.validate(AppShortcuts.bindingKey, binding.dictionary)
                    NSApplication.shared.setActivationPolicy(.prohibited) // Menu conflict adapter without a window or Dock icon.
                    try shortcuts.setBinding(binding)
                } else { throw AppUpdateError("未知命令或参数；用 hydromac \(verb) --help 查看。") }
                HydroSettings.notifyChanged()
            }
            body["startup_calculator"] = HydroPreferences.calculator
            body["enabled"] = shortcuts.enabled; body["binding"] = shortcuts.binding.dictionary
            body["description"] = shortcuts.descriptionText; body["search_shortcut"] = "⌘K"
        } catch { code = 2; body["ok"] = false; body["error"] = ["code": "usage", "message": error.localizedDescription] }
        if json { print(String(decoding: (try? JSONSerialization.data(withJSONObject: body, options: [.sortedKeys])) ?? Data(), as: UTF8.self)) }
        else if code == 0 { print("启动计算器：\(HydroPreferences.calculator) · \(body["description"] ?? "")") }
        else { fputs("\(body["error"] ?? "参数错误")\n", stderr) }
        return code
    }
}
