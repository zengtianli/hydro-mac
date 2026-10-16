# HydroMac Water-Resources Toolbox

[中文](README.md) | **English**

A native macOS toolbox for water-resources calculations: **SwiftUI frontend + Rust calculation backend**, self-contained in a single .app and usable offline.

> A native macOS toolbox for water-resources engineering: 8 calculators (annual water-resources report query, river pollutant-carrying capacity, water-efficiency assessment, reservoir dispatch, river-district dispatch, irrigation demand, rainfall pipeline, geocoding) built as a SwiftUI shell driving a vendored Rust CLI over a JSON contract. Apple Silicon, macOS 14+. All sample data is synthetic.

## 8 calculators

| Calculator | Description |
|---|---|
| Annual water-resources report query | Query and compare annual water supply, water use, and socioeconomic indicators |
| Pollutant-carrying capacity | Capacity of river functional zones using a one-dimensional decay model, with Excel input support |
| Water-efficiency assessment | Three-level assessment of reclaimed-water use (large/small/point cycles, combined AHP + CRITIC weighting) |
| Reservoir-system dispatch | Joint dispatch simulation for two reservoirs (annual regulation, rule curves + special storage volumes) |
| River-district dispatch model | Water-balance dispatch across multiple zones in a plain river district (19-zone example) |
| Irrigation demand | Crop water demand in irrigation districts (single/double-cropping rice, dryland, and miscellaneous land; daily water balance) |
| Rainfall data analysis | Complete pipeline for zonal rainfall, water withdrawals, and Gaihu evaporation (6 steps, TSV input) |
| Geocoding | Batch address↔coordinate conversion (Amap API; key supplied through the `AMAP_KEY` environment variable) + WGS-84/GCJ-02 coordinate conversion |

## Architecture

```
Sources/*.swift   SwiftUI 前端(swiftc 直编,无 xcodeproj)
      │  Foundation.Process + JSON(stdin → stdout 信封 {ok,data,error})
      ▼
cli/  (cargo)     hydro-cli —— 纯 Rust 计算核(serde/calamine/csv/chrono)
```

- The backend binary ships inside the .app at `Contents/Resources/hydro-cli`, with no runtime dependencies.
- The calculation core vendors the pure Rust calc module from the author’s upstream hydro project. This repository includes 20 golden numerical regression cases (`scripts/golden-test.sh`).
- Example data and golden cases are **all synthetic demonstration data**, with fictional zone/reservoir/water-user names and no real engineering records.

## Installation

**From a Release**: Download `HydroMac-<版本>-arm64.zip`, extract it, and drag `水利工具箱.app` into Applications. The app is ad-hoc signed without a paid developer certificate. Clear the quarantine flag once before first launch:

```bash
xattr -cr "/Applications/水利工具箱.app"
```

Alternatively, right-click → Open, then allow it in System Settings → Privacy & Security.

**Build from source** (requires Xcode + the Rust toolchain):

```bash
git clone https://github.com/zengtianli/hydro-mac.git
cd hydro-mac
./build.sh
```

## Development

```bash
( cd cli && cargo build --release )      # 编后端
echo '' | cli/target/release/hydro-cli annual sample   # 单测某个计算器
bash scripts/golden-test.sh              # 20 case 数值回归(改任何 calc 数学必须全 PASS)
./build.sh                               # 完整构建 → build/水利工具箱.app
```

System requirements: macOS 14+, Apple Silicon.

## Settings, shortcuts, iCloud and updates

Release 1.0.1 adds stable Developer ID signing, hardened runtime and Apple notarization while retaining the existing bundle identity and configuration. The original 1.0.0 archive used ad-hoc signing.

The author's local iteration installer is `./build.sh --install --prepared <signed.app>`. It requires a stable Developer ID and an actual current-source build receipt, verifies every bundle file, and retains the previous installation for recovery. Running apps and other products are left untouched. Default ad-hoc build candidates remain in `build/`; public release ZIPs use the documented manual installation flow.

The app menu opens native configuration and update settings. Select the startup calculator and customize or disable its app shortcut (default Command-R); Command-K searches tools. Reserved and conflicting keys are rejected.

Optional iCloud Drive configuration sync is off by default and uses the system Apple ID. Only the calculator preference, shortcut and update options migrate; credentials, permissions, paths and calculation data stay local. Import/export and local fallback are supported. Disabling sync preserves both copies.

Check for Updates reads this repository's official GitHub Releases and offers the download link. Automatic checking is optional and off by default. Public ad-hoc builds follow the documented download, extract and replace installation flow.

The bundled `Contents/Resources/bin/hydromac --help` exposes the same preferences, shortcuts, configuration and update adapters. The fixed acceptance script defaults to the installed app; explicit `--app` and `--receipt` arguments select an uninstalled candidate. Tests use the real product in disposable domains and offscreen views. Local isolated iCloud round trips do not establish Apple cross-device delivery.

`hydromac status` reads the version, eight calculators and account shape; `list --search <text>` searches the catalogue. `calc <calculator> <action> --input <JSON file or -> --json` uses the window's BackendClient and bundled Rust engine for imports, calculations and exports. Existing output requires `--force`. No product account is required for offline work; online mapping uses the user's Amap key, and errors omit credential-bearing URLs.

<!-- lightweight:start -->
## Resource use

| Download | Idle memory | Idle CPU | Annual sample query including engine process startup |
|---|---|---|---|
| **2.7 MB** (installed 6.8 MB) | **34.6 MB** | **0%** | **4.4 ms** |

Native SwiftUI with bundled Rust calculations; idle resources and a representative query are measured separately.

<sub>v1.0.0 (8) · Mac18,5 / Apple M6 / macOS 27.2 · Runtime figures describe a local source acceptance build and do not represent the public v1.0.0 release. Download bytes are measured separately from that public archive. Sample data is synthetic. · measured 2026-10-12. Measured on the listed version and device; the current version and measurement scope are described on this page. Memory uses phys_footprint; CPU is CPU time ÷ wall time over a 60-second sampling window; sizes in decimal MB. Raw data: [perf/lightweight.json](perf/lightweight.json).</sub>
<!-- lightweight:end -->

## License

[MIT](LICENSE)

## Relationship to the private version

This repo is a **one-way snapshot** of the author's private build. It is refreshed
only when the private version cuts a release — day-to-day private commits are not
mirrored, so this repo may lag behind at any given time. (Policy set 2026-08-10.)
