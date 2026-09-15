# 水利工具箱 HydroMac

**中文** | [English](README_EN.md)

原生 macOS 水利计算工具箱：**SwiftUI 前端 + Rust 计算后端**，单 .app 自包含，离线可用。

> 原生 macOS 水利工程工具箱，提供 8 个计算器：水资源年报查询、河道纳污能力、水效评估、水库调度、河区调度、灌溉需水、降雨处理与地理编码。SwiftUI 界面通过 JSON 接口调用随应用分发的 Rust CLI，支持 Apple Silicon 与 macOS 14+；所有示例数据均为合成数据。

## 8 个计算器

| 计算器 | 说明 |
|---|---|
| 水资源年报查询 | 供水量/用水量/社会经济指标逐年查询与对比 |
| 纳污能力计算 | 河道功能区纳污能力（一维衰减模型，支持 Excel 输入） |
| 水效评估 | 再生水利用三层评估（大/小/点循环，AHP + CRITIC 组合赋权） |
| 水库群调度 | 双库联合调度模拟（年调节，调度线 + 特殊库容） |
| 河区调度模型 | 平原河区多分区水量平衡调度（19 分区示例） |
| 灌溉需水计算 | 灌区作物需水（单双季稻/旱地/杂地，逐日水量平衡） |
| 降雨数据分析 | 分区降雨-取水-概湖蒸发全管线（6 步，TSV 输入） |
| 地理编码 | 地址↔坐标批量互转（高德 API，key 走环境变量 `AMAP_KEY`）+ WGS-84/GCJ-02 坐标系转换 |

## 架构

```
Sources/*.swift   SwiftUI 前端(swiftc 直编,无 xcodeproj)
      │  Foundation.Process + JSON(stdin → stdout 信封 {ok,data,error})
      ▼
cli/  (cargo)     hydro-cli —— 纯 Rust 计算核(serde/calamine/csv/chrono)
```

- 后端二进制随 .app 走（嵌 `Contents/Resources/hydro-cli`），无运行时依赖。
- 计算核 vendored 自作者上游 hydro 项目的纯 Rust calc；本仓自带 20 个 golden 数值回归用例（`scripts/golden-test.sh`）。
- 示例数据与 golden 用例**均为合成演示数据**（虚构的分区/水库/取水户名），不含任何真实工程台账。

## 安装

**从 Release**：下载 `HydroMac-<版本>-arm64.zip`，解压后把 `水利工具箱.app` 拖入「应用程序」。app 为 adhoc 签名（无付费开发者证书），首次运行前清一次隔离标记：

```bash
xattr -cr "/Applications/水利工具箱.app"
```

或右键 → 打开，再到「系统设置 → 隐私与安全性」放行。

**从源码构建**（需要 Xcode + Rust toolchain）：

```bash
git clone https://github.com/zengtianli/hydro-mac.git
cd hydro-mac
./build.sh --install
```

## 开发

```bash
( cd cli && cargo build --release )      # 编后端
echo '' | cli/target/release/hydro-cli annual sample   # 单测某个计算器
bash scripts/golden-test.sh              # 20 case 数值回归(改任何 calc 数学必须全 PASS)
./build.sh                               # 完整构建 → build/水利工具箱.app
```

系统要求：macOS 14+，Apple Silicon。

## 许可

[MIT](LICENSE)

## 与私有版本的关系

本仓库是作者私有版本的**单向快照**。仅在私有版本发布时更新；日常私有提交不会镜像到这里，因此本仓库可能落后于私有版本。（此规则于 2026-08-10 确定。）
