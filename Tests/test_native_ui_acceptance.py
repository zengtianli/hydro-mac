"""Binding regressions use disposable bundles; the real UI stays in the fixed acceptor."""
import ast
import importlib.util
import json
from pathlib import Path
import plistlib
import subprocess
import sys

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(Path.home() / 'Apps/chapter/engine'))
import app_sop

spec = importlib.util.spec_from_file_location('hydromac_product_checks', ROOT / 'scripts/accept/product_checks.py')
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


@pytest.fixture
def product(tmp_path, monkeypatch):
    for name in ('SOP_CODE_BINDING_SHA256', 'SOP_BUILD_CURRENT'):
        monkeypatch.delenv(name, raising=False)
    files = {'Sources/App.swift': 'production', 'Tests/check.py': 'test',
             'scripts/accept/native_ui.py': 'acceptor', 'scripts/hydromac': 'shipped CLI'}
    for name, content in files.items():
        path = tmp_path / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_text(content)
    app = tmp_path / 'Product.app'
    binary = app / 'Contents/MacOS/HydroMac'; binary.parent.mkdir(parents=True); binary.write_bytes(b'product executable')
    icon = app / 'Contents/Resources/AppIcon.icns'; icon.parent.mkdir(); icon.write_bytes(b'product icon')
    info = dict(CFBundleIdentifier=checks.PRODUCT, CFBundleExecutable='HydroMac', CFBundleVersion='32',
                CFBundleShortVersionString='1.0.1', CFBundleIconFile='AppIcon')
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
    patterns = ['Sources/**', 'Tests/**', 'scripts/accept/**', 'scripts/hydromac']
    registration = dict(id='hydro-mac-oss', repo=tmp_path, bundle_id=checks.PRODUCT, form='mac', sop={'source': patterns})
    receipt = tmp_path / 'perf/build-receipt.json'; receipt.parent.mkdir()
    receipt.write_text(json.dumps(dict(schema_version=1, built_at='2026-10-17T00:00:00Z', build_command_sha256='a' * 64,
        artifact=app_sop.artifact_snapshot(app), source=app_sop.app_source_snapshot(registration, patterns, include_files=True))))
    monkeypatch.setattr(app_sop, 'load_apps', lambda name: [registration])
    return app, receipt, registration


def queue_binding(monkeypatch, current='1'):
    monkeypatch.setenv('SOP_CODE_BINDING_SHA256', 'a' * 64)
    monkeypatch.setenv('SOP_BUILD_CURRENT', current)


def test_queue_binding_is_authoritative_without_receipt_file_walk(product, monkeypatch):
    app, receipt, _ = product
    queue_binding(monkeypatch)
    monkeypatch.setattr(app_sop, 'verify_build_receipt', lambda *a, **kw: pytest.fail('Queue already verified production'))
    binding = checks.code_binding(app, receipt)
    assert binding == dict(sha256='a' * 64, build_current=True, method='chapter_environment', detail='Chapter SOP_BUILD_CURRENT=1')


def test_direct_cli_reuses_original_verifier_and_ignores_acceptor_churn(product):
    app, receipt, registration = product
    (registration['repo'] / 'scripts/accept/native_ui.py').write_text('fixed acceptance')
    (registration['repo'] / 'Tests/check.py').write_text('fixed fixture')
    binding = checks.code_binding(app, receipt)
    assert binding['build_current'], binding['detail']
    assert binding['method'] == 'chapter_receipt_verifier' and len(binding['sha256']) == 64


def test_build_producer_covers_icon_sources_without_acceptor_inputs(product):
    app, receipt, registration = product
    for name in ('AppIcon.icns', 'AppIcon.png', 'AppIcon.iconset/icon_128x128.png'):
        path = registration['repo'] / 'icon' / name; path.parent.mkdir(parents=True, exist_ok=True); path.write_bytes(b'icon source')
    registration['sop']['source'].append('icon/**')
    tree = ast.parse((ROOT / 'scripts/test_product.py').read_text())
    patterns = next(ast.literal_eval(node.value) for node in tree.body if isinstance(node, ast.Assign)
                    and any(isinstance(target, ast.Name) and target.id == 'patterns' for target in node.targets))
    source = app_sop.app_source_snapshot(registration, patterns, include_files=True)
    assert {'icon/AppIcon.icns', 'icon/AppIcon.png', 'icon/AppIcon.iconset/icon_128x128.png'} <= source['files'].keys()
    assert 'scripts/accept/native_ui.py' not in source['files'] and 'Tests/check.py' not in source['files']
    body = json.loads(receipt.read_text()); body['source'] = source; receipt.write_text(json.dumps(body))
    binding = checks.code_binding(app, receipt)
    assert binding['build_current'], binding['detail']


@pytest.mark.parametrize('change', ['changed', 'added', 'deleted', 'shipped_cli', 'truncated_receipt'])
def test_direct_cli_rejects_unproved_production(product, change):
    app, receipt, registration = product
    source = registration['repo'] / 'Sources/App.swift'
    if change == 'changed': source.write_text('new production')
    elif change == 'added': source.with_name('New.swift').write_text('new compiled source')
    elif change == 'deleted': source.unlink()
    elif change == 'shipped_cli': (registration['repo'] / 'scripts/hydromac').write_text('new shipped CLI')
    else:
        body = json.loads(receipt.read_text()); del body['source']['files']['Sources/App.swift']
        receipt.write_text(json.dumps(body))
    binding = checks.code_binding(app, receipt)
    assert not binding['build_current'] and binding['detail']


@pytest.mark.parametrize('field,value', [('SOP_BUILD_CURRENT', None), ('SOP_CODE_BINDING_SHA256', None),
                                      ('SOP_BUILD_CURRENT', 'true'), ('SOP_CODE_BINDING_SHA256', 'invalid')])
def test_incomplete_queue_contract_cannot_fall_back(product, monkeypatch, field, value):
    app, receipt, _ = product
    queue_binding(monkeypatch)
    if value is None: monkeypatch.delenv(field)
    else: monkeypatch.setenv(field, value)
    with pytest.raises(ValueError, match='invalid Chapter'):
        checks.code_binding(app, receipt)


def test_queue_stale_build_rejected_before_signature_or_ui(product, monkeypatch):
    app, receipt, _ = product
    queue_binding(monkeypatch, '0')
    monkeypatch.setattr(subprocess, 'run', lambda *a, **kw: pytest.fail('No process should start for stale production'))
    with pytest.raises(ValueError, match='not current.*SOP_BUILD_CURRENT=0'):
        checks.bundle_binding(app, receipt)


@pytest.mark.parametrize('change,detail', [('version', 'version'), ('build', 'build'), ('binary', 'sha256'), ('icon', 'icon SHA256')])
def test_current_queue_binding_does_not_hide_actual_artifact_mismatch(product, monkeypatch, change, detail):
    app, receipt, _ = product
    queue_binding(monkeypatch)
    if change in ('version', 'build'):
        path = app / 'Contents/Info.plist'; info = plistlib.loads(path.read_bytes())
        info['CFBundleShortVersionString' if change == 'version' else 'CFBundleVersion'] = '31'
        path.write_bytes(plistlib.dumps(info))
    else:
        path = app / ('Contents/MacOS/HydroMac' if change == 'binary' else 'Contents/Resources/AppIcon.icns')
        path.write_bytes(b'other product bytes')
    monkeypatch.setattr(subprocess, 'run', lambda *a, **kw: pytest.fail('No process should start for mismatched artifact'))
    with pytest.raises(ValueError, match=detail): checks.bundle_binding(app, receipt)


def test_current_queue_binding_still_requires_real_signature(product, monkeypatch):
    app, receipt, _ = product
    queue_binding(monkeypatch)
    calls = []
    def invalid_signature(argv, **kwargs):
        calls.append(argv)
        raise subprocess.CalledProcessError(1, argv, stderr=b'invalid signature')
    monkeypatch.setattr(subprocess, 'run', invalid_signature)
    with pytest.raises(subprocess.CalledProcessError): checks.bundle_binding(app, receipt)
    assert calls == [['codesign', '--verify', '--deep', '--strict', str(app)]]


def test_fixed_acceptor_reports_mismatch_before_native_launch(product, monkeypatch):
    app, receipt, registration = product
    path = app / 'Contents/Info.plist'; info = plistlib.loads(path.read_bytes()); info['CFBundleVersion'] = '31'
    path.write_bytes(plistlib.dumps(info))
    monkeypatch.setattr(checks, 'ROOT', registration['repo'])
    monkeypatch.setenv('SOP_OUT_DIR', str(registration['repo'] / 'out'))
    monkeypatch.setattr(sys, 'argv', ['native_ui.py', '--app', str(app), '--receipt', str(receipt)])
    calls = []
    monkeypatch.setattr(subprocess, 'run', lambda argv, **kw: calls.append(argv))
    assert checks.main('native_ui') == 1
    detail = json.loads((registration['repo'] / 'out/native_ui.detail.json').read_text())
    assert 'expected 32, found 31' in detail['error']
    assert detail['checks'] == {'execution_completed': False}
    assert all(argv[:2] == ['defaults', 'delete'] for argv in calls)
