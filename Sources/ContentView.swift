import SwiftUI

/// 计算器登记表 —— 8 个水利计算器。
struct Calculator: Identifiable, Hashable {
    let key: String
    let name: String
    let icon: String
    let ready: Bool
    var id: String { key }
}

let CALCULATORS: [Calculator] = [
    Calculator(key: "annual", name: "水资源年报查询", icon: "doc.text.magnifyingglass", ready: true),
    Calculator(key: "capacity", name: "纳污能力计算", icon: "drop.triangle", ready: true),
    Calculator(key: "efficiency", name: "水效评估", icon: "gauge.with.dots.needle.67percent", ready: true),
    Calculator(key: "reservoir", name: "水库群调度", icon: "water.waves", ready: true),
    Calculator(key: "district", name: "河区调度模型", icon: "point.3.connected.trianglepath.dotted", ready: true),
    Calculator(key: "irrigation", name: "灌溉需水计算", icon: "leaf", ready: true),
    Calculator(key: "rainfall", name: "降雨数据分析", icon: "cloud.rain", ready: true),
    Calculator(key: "geocode", name: "地理编码", icon: "mappin.and.ellipse", ready: true),
]

struct ContentView: View {
    @State private var selection: Calculator? = CALCULATORS.first
    @State private var showPalette = false

    var body: some View {
        NavigationSplitView {
            List(selection: $selection) {
                Section("本地计算器") {
                    ForEach(CALCULATORS) { calc in
                        Label {
                            HStack {
                                Text(calc.name)
                                if !calc.ready {
                                    Spacer()
                                    Text("建设中").font(.caption2).foregroundStyle(.tertiary)
                                }
                            }
                        } icon: {
                            Image(systemName: calc.icon)
                                .foregroundStyle(calc.ready ? Color.accentColor : Color.secondary)
                        }
                        .tag(calc)
                    }
                }
            }
            .navigationTitle("水利工具箱")
            .frame(minWidth: 210)
        } detail: {
            if let selection {
                if selection.key == "annual" {
                    AnnualView()
                } else if selection.key == "capacity" {
                    CapacityView()
                } else if selection.key == "efficiency" {
                    EfficiencyView()
                } else if selection.key == "reservoir" {
                    ReservoirView()
                } else if selection.key == "district" {
                    DistrictView()
                } else if selection.key == "irrigation" {
                    IrrigationView()
                } else if selection.key == "rainfall" {
                    RainfallView()
                } else if selection.key == "geocode" {
                    GeocodeView()
                } else {
                    placeholder(selection)
                }
            } else {
                Text("选择一个计算器").foregroundStyle(.secondary)
            }
        }
        .commandPalette(items: paletteItems, isPresented: $showPalette)
    }

    // ⌘K 命令面板:8 计算器,选中即跳转。
    private var paletteItems: [PaletteItem] {
        CALCULATORS.map { c in
            PaletteItem(id: "calc-\(c.key)", title: c.name,
                        subtitle: c.ready ? "本地计算器" : "本地计算器 · 建设中",
                        icon: c.icon, keywords: c.key) { selection = c }
        }
    }

    private func placeholder(_ calc: Calculator) -> some View {
        VStack(spacing: 14) {
            Image(systemName: calc.icon).font(.system(size: 44)).foregroundStyle(.tertiary)
            Text(calc.name).font(.title2.bold())
            Text("Rust 计算核已就绪(vendored),SwiftUI 视图建设中")
                .foregroundStyle(.secondary)
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity)
        .navigationTitle(calc.name)
    }
}
