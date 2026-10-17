# 本轮 native_ui 失败修复（2026-10-17）

本轮仅修 delivery-native_ui；owner 为 Chapter 已登记身份 `chapter-task:41fafbbf3ad64543b60ed2179d2e04b8` / PID 63127，写前沿该身份核 claims，不另登记或释放。只修改 `scripts/accept/product_checks.py` 的共用产物/源码校验和错误报告、`scripts/test_product.py` 的回执输入列表、新增 `Tests/test_native_ui_acceptance.py`，以及本交接；保留其他会话的 README、既有 handoff、perf、dist、素材、暂存和后台。

原失败日志为 `/Users/tianli/Library/Application Support/app-sop/workbench/logs/fc5e3f2ff3244e16a473fe46a26a8c57.log`；原验收 `perf/acceptance/run-ecj79rtr/native_ui.json` 只有 `execution_completed=false`，raw 的 binding/cases 为空、error 为空，尚未执行界面自检。现查已装包为 1.0.1 (31)，默认 `perf/build-receipt.json` 及现有 `build/release/水利工具箱.app` 为 (32)；原无消息 assert 丢失了实际不匹配原因。

完成条件：生产源码绑定消费 Chapter 的 `SOP_CODE_BINDING_SHA256` / `SOP_BUILD_CURRENT`，无这两个变量时复用原只读 receipt verifier；保留实际 bundle、版本、可执行文件和签名核验；回归证明验收脚本变化不误伤、生产输入变化及真实产物不匹配仍拒绝；候选包离屏 native_ui 检查通过后本地提交。原候选来源集合不完整，故使用共享 `sim_lane.BuildSlot(wait=0)` 在独立目录真实构建一次，不覆盖现有 build/release 或 perf 回执；离屏验收不占模拟器/性能安静门、不启动本人 App。证据由原脚本写入 `build/accept/native-ui-repair-20261017/`。不装机、推送、发版、部署或改授权，固定复验交原 jobs 队列。

Chapter 的原 verifier 还查出默认回执未覆盖 `sop.source` 的 `icon/AppIcon.png` 及 `icon/AppIcon.iconset/*.png`，生产内容未变但来源集合不完整。已修正原构建生产者的 patterns 为 `icon/**`，并移除不属于生产输入的 `scripts/accept/*.py`。旧回执不修改、不补写通过；下一次真实构建沿原生产者覆盖完整输入。

最终验证与边界：

- `PYTHONDONTWRITEBYTECODE=1 /Users/tianli/Dev/.venv/bin/python -m pytest -n auto -p no:cacheprovider --basetemp=build/accept/native-ui-repair-20261017/pytest Tests/test_native_ui_acceptance.py -q`：19 passed，4.70 秒。覆盖 Chapter 环境优先、缺失/坏环境拒绝、直接调用原 verifier、验收/测试脚本变化、生产文件新增/删除/改变、包内 CLI 变化、截断回执、PNG 图标输入覆盖、真实版本/二进制/图标/签名不匹配以及固定入口的明确错误证据。
- 实际隔离构建位于 `build/accept/native-ui-repair-20261017/workspace/`，使用原 build.sh（Swift/Rust 自包含包），并由 Chapter 原 `app_sop.build_receipt` 在真实构建前后绑定生产输入，生成同目录上级 `build-receipt.json`；`verify_build_receipt(..., code_only=True)` 返回 true。没有给旧包补写来源，没有调用 prepare_release、签名发行/公证或装机入口；只含原 build.sh 的本地 ad-hoc 构建签名。
- 固定脚本 `scripts/accept/native_ui.py --app "$PWD/build/accept/native-ui-repair-20261017/workspace/build/水利工具箱.app" --receipt "$PWD/build/accept/native-ui-repair-20261017/build-receipt.json"`，配 `SOP_OUT_DIR="$PWD/build/accept/native-ui-repair-20261017/candidate-check"`：退出 0、44/44 actual checks；真实 raw 为 `build/accept/native-ui-repair-20261017/candidate-check/native_ui-04309c39854b4c7da47dd3940d30ebe1/raw.json`，SHA256 `3489788d354e02ca7117af396cc3d1d841b336929392b3a0143287af7f45a5b5`。涵盖八计算器后端、离屏渲染、实际 mounted 降雨按钮与六步管线、搜索导航、关闭、无可见窗口/激活；已核读 `ui/native-ui-rainfall-actions.png` 的侧栏和结果文字。
- 已装路径另用相同固定脚本、独立 `installed-check/` 输出回读，退出 1，明确 `ValueError: Product/receipt mismatch (build): expected 32, found 31`。这是实际产物差异，不能返回 75 或伪报通过。已装可执行文件最终仍为构建 31 / SHA256 `4e05a5408cd4eb66cce14d60080a6a0cf4ddfcd7ea850df0441605a54f56fd6e`，与开工读数相同。候选通过不代表当前已装包通过。
- 三个 Python 文件语法检查及 `git diff --check` 通过；本地提交只包含本轮两个脚本、新增测试与本交接，不提交机器输出，不触碰其他会话暂存和改动，也不释放 Chapter 的 claim。

本轮仓内修复和候选验收已完成。默认已装验收仍需处理原构建 31 与回执 32 的差异，本轮授权明确不装机，保留给 Chapter 原 jobs/装机流程；不新建任务或自动重试循环。用户在本仓 CLI 接手可先用 `chapter jobs --app hydro-mac-oss --active --json` 读原队列，实际已装包与完整回执一致后再用 `chapter sop accept --app hydro-mac-oss --check native_ui --json` 固定复验；本轮不调用该队列的装机、发布或部署动作。
