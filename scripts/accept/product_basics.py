#!/usr/bin/env python3
"""Actual HydroMac basics: default installed bundle, or an explicitly source-bound candidate.

Only disposable identities, preference domains and directories are changed. No user UI/input,
production configuration/iCloud, real installation, notarization or release is performed.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import sys
import time
import uuid

from product_checks import bundle_binding, evidence_output

ROOT = Path(__file__).resolve().parents[2]
PRODUCT = 'io.github.zengtianli.HydroMac'
FEATURES = ('settings', 'shortcuts', 'icloud', 'updates')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')


def execute(argv, **kw):
    return subprocess.run(list(map(str, argv)), capture_output=True, text=True, timeout=kw.pop('timeout', 60), **kw)


def owner_snapshot():
    prefs = subprocess.run(['/usr/bin/defaults', 'export', PRODUCT, '-'], capture_output=True, timeout=15)
    canonical = plistlib.dumps(plistlib.loads(prefs.stdout), sort_keys=True) if prefs.returncode == 0 else b''
    result = {'defaults_sha256': hashlib.sha256(canonical).hexdigest(), 'installed': {}, 'configuration': {}, 'cloud_entries': {}}
    app = Path('/Applications/水利工具箱.app')
    if app.exists():
        result['installed'] = {str(p.relative_to(app)): sha(p) for p in app.rglob('*') if p.is_file() and not p.is_symlink()}
    support = Path.home() / 'Library/Application Support/TianliApps/Configuration' / PRODUCT
    if support.exists():
        result['configuration'] = {str(p.relative_to(support)): sha(p) for p in support.rglob('*') if p.is_file() and not p.is_symlink()}
    cloud = Path.home() / 'Library/Mobile Documents/com~apple~CloudDocs/TianliApps/Updates' / PRODUCT
    if cloud.exists():
        result['cloud_entries'] = {str(p.relative_to(cloud)): [p.stat().st_size, p.stat().st_mtime_ns] for p in cloud.rglob('*') if p.is_file()}
    settings = Path.home() / 'Library/Mobile Documents/com~apple~CloudDocs/TianliApps/Settings' / (PRODUCT + '.json')
    result['cloud_settings'] = [settings.stat().st_size, settings.stat().st_mtime_ns] if settings.exists() else None
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--app', type=Path, default=os.environ.get('HYDROMAC_BASICS_APP', '/Applications/水利工具箱.app'))
    parser.add_argument('--receipt', type=Path, default=os.environ.get('HYDROMAC_BASICS_RECEIPT', str(ROOT / 'perf/build-receipt.json')))
    args = parser.parse_args()
    app, receipt_path = args.app.expanduser().resolve(), args.receipt.expanduser().resolve()
    if 'SOP_PRODUCT_BASICS_SPEC' not in os.environ:
        sys.path.insert(0, str(Path.home() / 'Apps/chapter/engine'))
        import app_sop
        import product_basics
        spec = product_basics.spec(app_sop.load_apps('hydro-mac-oss')[0])
    else:
        spec = json.loads(os.environ['SOP_PRODUCT_BASICS_SPEC'])
    assert spec['app_id'] == os.environ.get('SOP_APP_ID', 'hydro-mac-oss') == 'hydro-mac-oss'
    out, evidence_root = evidence_output('perf/acceptance')
    run = out / ('product-basics-' + uuid.uuid4().hex)
    run.mkdir(parents=True)
    suite = 'test.hydromac.basics.' + uuid.uuid4().hex
    env = {k: v for k, v in os.environ.items() if k not in ('AMAP_KEY', 'AMAP_API_KEY', 'HYDROMAC_APP', 'HYDROMAC_PREFS_DOMAIN') and not k.startswith(('APP_LIFECYCLE_', 'HYDROMAC_SELFTEST_', 'HYDROMAC_BASICS_'))}
    support, cloud, native_out = run / 'support', run / 'cloud', run / 'native'
    support.mkdir(); cloud.mkdir()
    env.update(APP_LIFECYCLE_SUPPORT_DIR=str(support), APP_LIFECYCLE_CLOUD_DIR=str(cloud),
               APP_LIFECYCLE_NO_RELAUNCH='1', HYDROMAC_BASICS_OUT=str(native_out), HYDROMAC_PREFS_DOMAIN=suite,
               PYTHONDONTWRITEBYTECODE='1')
    checks, cases, binding = {}, [], {}
    before = owner_snapshot(); write(run / 'owner-before.json', before)
    error = None
    scope = 'installed' if app.is_relative_to(Path('/Applications')) else 'candidate_not_installed'
    try:
        binding, info = bundle_binding(app, receipt_path)
        artifact = binding['artifact']
        binding.update(scope=scope, receipt=str(receipt_path), acceptor_sha256=sha(Path(__file__)))
        for feature in FEATURES:
            checks[feature + '.bundle_receipt_matches'] = True
            checks[feature + '.current_source_matches'] = binding['code_binding']['build_current']
            checks[feature + '.signature_valid'] = True
        clone = support / 'HydroMac.app'
        shutil.copytree(app, clone, symlinks=True)
        assert sha(clone / artifact['executable']) == artifact['sha256']
        info.update(CFBundleIdentifier=suite, LSUIElement=True)
        (clone / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
        signed_copy = execute(['codesign', '--force', '--deep', '--sign', '-', clone])
        assert signed_copy.returncode == 0, signed_copy.stderr
        # A genuine newer App package is built from the same candidate, not a guessed manifest/stub asset.
        newer = run / 'newer/HydroMac.app'; newer.parent.mkdir()
        shutil.copytree(clone, newer, symlinks=True)
        newer_info = dict(info, CFBundleShortVersionString='99.0.0', CFBundleVersion=str(int(info['CFBundleVersion']) + 1))
        (newer / 'Contents/Info.plist').write_bytes(plistlib.dumps(newer_info))
        assert execute(['codesign', '--force', '--deep', '--sign', '-', newer]).returncode == 0
        channel = run / 'download-fixture'; channel.mkdir(parents=True)
        archive = channel / 'isolated-newer.zip'
        assert execute(['/usr/bin/ditto', '-c', '-k', '--keepParent', newer, archive]).returncode == 0
        release = {'tag_name': 'v99.0.0', 'draft': False, 'prerelease': False,
                   'html_url': 'https://github.com/zengtianli/hydro-mac/releases/tag/v99.0.0',
                   'assets': [{'name': 'HydroMac-arm64.zip', 'size': archive.stat().st_size,
                               'digest': 'sha256:' + sha(archive),
                               'browser_download_url': 'https://github.com/zengtianli/hydro-mac/releases/download/v99.0.0/HydroMac-arm64.zip'}]}
        write(support / 'good-github-release.json', release)
        print('source-bound ' + scope + ': offscreen native settings/shortcut/sync/update checks', flush=True)
        native = execute([clone / 'Contents/MacOS/HydroMac', '--product-basics-self-test'], env=env, cwd=ROOT, timeout=180)
        (run / 'native.log').write_text(native.stdout + native.stderr)
        report = json.loads((native_out / 'native.json').read_text())
        checks.update(report['checks']); cases.extend(report['cases'])
        for feature in FEATURES: checks[feature + '.native_exit_success'] = native.returncode == 0
        binding['native'] = {'path': str((native_out / 'native.json').relative_to(evidence_root)), 'sha256': sha(native_out / 'native.json')}
        binding['screenshots'] = [{'path': str((native_out / p).relative_to(evidence_root)), 'sha256': sha(native_out / p)} for p in report['screenshots']]

        # Public ad-hoc distribution legitimately requires downloading/extracting the App.
        # Exercise that documented manual installation on the real disposable package.
        cli = clone / 'Contents/Resources/bin/hydromac'
        def call(words):
            result = execute([cli, *words, '--json'], env=env, cwd=ROOT, timeout=90)
            body = json.loads(result.stdout)
            cases.append({'argv': words, 'exit': result.returncode, 'response': body, 'stderr': result.stderr})
            return result.returncode, body
        exported = run / 'before-upgrade.json'
        code, _ = call(['config', 'export', '-o', str(exported)])
        assert code == 0
        portable_before = exported.read_bytes()
        old_main = sha(clone / artifact['executable'])
        code, refused = call(['update', 'install', '--yes'])
        checks['updates.public_manual_download_boundary'] = (code == 1 and refused.get('error', {}).get('code') == 'manual_install'
            and refused.get('download_url') == release['assets'][0]['browser_download_url'] and sha(clone / artifact['executable']) == old_main)
        extracted = run / 'manual-download'; extracted.mkdir()
        assert execute(['/usr/bin/ditto', '-x', '-k', archive, extracted]).returncode == 0
        downloaded = extracted / 'HydroMac.app'
        assert execute(['codesign', '--verify', '--deep', '--strict', downloaded]).returncode == 0
        assert clone.is_relative_to(support) and info['CFBundleIdentifier'] == suite
        shutil.rmtree(clone)
        assert execute(['/usr/bin/ditto', downloaded, clone]).returncode == 0
        installed_info = plistlib.loads((clone / 'Contents/Info.plist').read_bytes())
        _, status = call(['preferences', 'status'])
        after_file = run / 'after-upgrade.json'; after_code, _ = call(['config', 'export', '-o', str(after_file)])
        credential = execute(['/usr/bin/defaults', 'read', suite, 'geocode.amapKey'])
        upgrade_ok = (installed_info['CFBundleVersion'] == newer_info['CFBundleVersion'] and installed_info['CFBundleShortVersionString'] == '99.0.0'
                      and status.get('startup_calculator') == 'irrigation' and after_code == 0 and after_file.read_bytes() == portable_before
                      and credential.returncode == 0 and credential.stdout.strip() == 'isolated-test-credential'
                      and execute(['codesign', '--verify', '--deep', '--strict', clone]).returncode == 0)
        checks['updates.actual_isolated_manual_upgrade_preserves_data'] = upgrade_ok
        checks['updates.data_preservation'] = checks.get('updates.data_preservation') is True and upgrade_ok
    except Exception as exc:
        error = f'{type(exc).__name__}: {exc}'
        if isinstance(exc, subprocess.CalledProcessError) and exc.stderr:
            error += ': ' + (exc.stderr.decode(errors='replace') if isinstance(exc.stderr, bytes) else exc.stderr).strip()
        for feature in FEATURES: checks[feature + '.execution_completed'] = False
    finally:
        execute(['/usr/bin/defaults', 'delete', suite])
        after = owner_snapshot(); write(run / 'owner-after.json', after)
        for feature in FEATURES: checks[feature + '.user_data_and_installed_bytes_unchanged'] = before == after
    raw = {'schema_version': 1, 'app_id': 'hydro-mac-oss', 'spec_digest': spec['digest'], 'checks': checks, 'cases': cases,
           'binding': binding, 'error': error, 'checked_at': time.time(),
           'scope': scope + '; real product copy/native controls/actions/fresh processes/isolated cloud and public GitHub transport fixture and genuine newer bundle manual installation; Apple cross-device delivery untested'}
    original = run / 'raw.json'; write(original, raw)
    records = {}
    for feature in FEATURES:
        keys = [k for k in checks if k.startswith(feature + '.')]
        required = spec['features'][feature]['required_checks']
        passed = bool(keys) and all(checks[k] is True for k in keys) and all(checks.get(k) is True for k in required)
        records[feature] = {'result': 'passed' if passed else 'failed', 'method': 'real_product', 'evidence': str(original.relative_to(evidence_root)),
                            'evidence_sha256': sha(original), 'checks': keys}
    passed = all(r['result'] == 'passed' for r in records.values())
    detail = {'schema_version': 1, 'app_id': 'hydro-mac-oss', 'spec_digest': spec['digest'], 'features': records,
              'summary': ('候选包（未安装）' if scope == 'candidate_not_installed' else '已装包') + ('四项基本功能通过' if passed else '四项基本功能失败') +
              f'；{sum(v is True for v in checks.values())}/{len(checks)} 项真实检查；iCloud 为隔离目录链路', 'scope': raw['scope'], 'error': error}
    write(out / 'product_basics.detail.json', detail)
    print(detail['summary'], flush=True)
    if not passed: print('failed:', [k for k, v in checks.items() if v is not True], 'error:', error, flush=True)
    return 0 if passed else 1


if __name__ == '__main__':
    raise SystemExit(main())
