#!/usr/bin/env python3
"""Real packaged-product acceptance, isolated files/domains and unchanged golden originals."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import sys
import uuid
import zipfile

ROOT = Path(__file__).resolve().parents[2]
PRODUCT = 'io.github.zengtianli.HydroMac'
CALCULATORS = ['annual', 'capacity', 'efficiency', 'reservoir', 'district', 'irrigation', 'rainfall', 'geocode']
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()

def write(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, ensure_ascii=False, indent=2) + '\n')

def same(a, b):
    if isinstance(a, bool) or isinstance(b, bool): return a is b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)): return math.isclose(a, b, rel_tol=1e-12, abs_tol=1e-12)
    if isinstance(a, dict) and isinstance(b, dict): return a.keys() == b.keys() and all(same(a[k], b[k]) for k in a)
    if isinstance(a, list) and isinstance(b, list): return len(a) == len(b) and all(same(x, y) for x, y in zip(a, b))
    return a == b

def code_binding(app, receipt_path):
    """Use Chapter's production binding, including its complete source-set check."""
    digest = os.environ.get('SOP_CODE_BINDING_SHA256')
    current = os.environ.get('SOP_BUILD_CURRENT')
    if digest is not None or current is not None:
        if not re.fullmatch(r'[0-9a-f]{64}', digest or '') or current not in ('0', '1'):
            raise ValueError('Incomplete or invalid Chapter production binding environment')
        return {'sha256': digest, 'build_current': current == '1', 'method': 'chapter_environment',
                'detail': 'Chapter SOP_BUILD_CURRENT=' + current}
    # Direct CLI/candidate acceptance has no queue environment. Reuse the same
    # read-only authority; do not maintain another list of non-production paths.
    sys.path.insert(0, str(Path.home() / 'Apps/chapter/engine'))
    import app_sop
    product = app_sop.load_apps('hydro-mac-oss')[0]
    valid, reason = app_sop.verify_build_receipt(product, app, receipt_path=receipt_path, code_only=True)
    _, business = app_sop.business_code_files(product, {})
    return {'sha256': app_sop.digest_value([business]), 'build_current': valid,
            'method': 'chapter_receipt_verifier', 'detail': reason}

def bundle_binding(app, receipt_path):
    receipt = json.loads(receipt_path.read_text()); artifact = receipt['artifact']
    info = plistlib.loads((app / 'Contents/Info.plist').read_bytes())
    expected = {'bundle_id': PRODUCT, 'version': artifact['version'], 'build': str(artifact['build']),
                'executable': artifact['executable'], 'sha256': artifact['sha256']}
    observed = {'bundle_id': info['CFBundleIdentifier'], 'version': info['CFBundleShortVersionString'],
                'build': str(info['CFBundleVersion']), 'executable': 'Contents/MacOS/' + info['CFBundleExecutable']}
    if artifact['bundle_id'] != PRODUCT:
        raise ValueError('Receipt belongs to another product: ' + str(artifact['bundle_id']))
    observed['sha256'] = sha(app / observed['executable'])
    for field, value in expected.items():
        if observed[field] != value:
            raise ValueError(f'Product/receipt mismatch ({field}): expected {value}, found {observed[field]}')
    if artifact.get('icon'):
        icon = artifact['icon']
        if sha(app / icon['path']) != icon['sha256']:
            raise ValueError('Product/receipt mismatch (icon SHA256)')
    binding = {'app': str(app), 'artifact': artifact, 'receipt_sha256': sha(receipt_path),
               'source_sha256': receipt['source']['sha256'], 'source_commit': receipt['source']['commit'],
               'code_binding': code_binding(app, receipt_path)}
    if not binding['code_binding']['build_current']:
        raise ValueError('Production source/build binding is not current: ' + binding['code_binding']['detail'])
    subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True, capture_output=True)
    return binding, info

def main(check=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--check', default=check, choices=['functionality', 'recovery', 'privacy', 'native_ui'])
    parser.add_argument('--app', type=Path, default=os.environ.get('HYDROMAC_ACCEPT_APP', '/Applications/水利工具箱.app'))
    parser.add_argument('--receipt', type=Path, default=os.environ.get('HYDROMAC_ACCEPT_RECEIPT', str(ROOT / 'perf/build-receipt.json')))
    args = parser.parse_args(); name = args.check
    out = Path(os.environ.get('SOP_OUT_DIR', str(ROOT / 'build/accept/checks'))).resolve()
    assert out.is_relative_to(ROOT)
    run = out / (name + '-' + uuid.uuid4().hex); run.mkdir(parents=True)
    checks, cases, binding, error = {}, [], {}, None
    env = {k: v for k, v in os.environ.items() if k not in ['AMAP_KEY', 'AMAP_API_KEY', 'HYDRO_CLI_PATH'] and not k.startswith(('APP_LIFECYCLE_', 'HYDROMAC_'))}
    domain = 'test.hydromac.ui.' + uuid.uuid4().hex
    env.update(APP_LIFECYCLE_SUPPORT_DIR=str(run / 'support'), APP_LIFECYCLE_CLOUD_DIR=str(run / 'cloud'), HYDROMAC_UI_OUT=str(run / 'ui'))
    try:
        app = args.app.resolve()
        binding, info = bundle_binding(app, args.receipt.resolve()); artifact = binding['artifact']
        checks['real_bundle_source_and_receipt'] = True
        clone = run / 'Product.app'; shutil.copytree(app, clone, symlinks=True)
        info.update(CFBundleIdentifier=domain, LSUIElement=True)
        (clone / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
        subprocess.run(['codesign', '--force', '--deep', '--sign', '-', str(clone)], check=True, capture_output=True)
        engine = clone / 'Contents/Resources/hydro-cli'; cli = clone / 'Contents/Resources/bin/hydromac'
        profile = run / 'offline.sb'; profile.write_text('(version 1) (allow default) (deny network*)\n')
        def invoke(calc, action, payload=None, agent=False):
            command = [str(cli), 'calc', calc, action, '--input', '-', '--json'] if agent else [str(engine), calc, action]
            done = subprocess.run(['sandbox-exec', '-f', str(profile), *command], input=json.dumps(payload or {}, ensure_ascii=False), capture_output=True, text=True, env=env, cwd=ROOT, timeout=45)
            body = json.loads(done.stdout)
            cases.append({'calculator': calc, 'action': action, 'agent': agent, 'exit': done.returncode, 'response': body, 'stderr': done.stderr})
            return done.returncode, body
        if name in ['functionality', 'privacy']:
            count = 0
            for case in sorted((ROOT / 'cli/golden').glob('*/case-*')):
                calc, action = (case / 'cmd').read_text().strip().split()
                code, body = invoke(calc, action, json.loads((case / 'input.json').read_text()))
                expected = json.loads((case / 'expected.json').read_text())
                checks['golden.' + str(case.relative_to(ROOT / 'cli/golden'))] = code == 0 and same(body, expected)
                count += 1
            checks['golden_count_20'] = count == 20
            for calc in CALCULATORS:
                code, body = invoke(calc, 'sample')
                checks[calc + '.offline_without_product_account'] = code == 0 and body.get('ok') is True and bool(body.get('data'))
            _, sample = invoke('annual', 'sample'); index = sample['data']
            for table in index['available_tables']:
                _, result = invoke('annual', 'query', {'index': index, 'input': {'table': table, 'cities': [], 'years': [], 'indicators': []}})
                checks['annual.query.' + table] = result['ok'] is True and bool(result['data']['rows']) and bool(result['data']['headers'])
            code, result = invoke('capacity', 'run', json.loads((ROOT / 'cli/golden/capacity/case-2/input.json').read_text()), agent=True)
            checks['agent_runs_same_calculation'] = code == 0 and result.get('ok') is True and same(result['data'], json.loads((ROOT / 'cli/golden/capacity/case-2/expected.json').read_text())['data'])
            if name == 'functionality':
                file = run / 'actual-results.xlsx'
                code, exported = invoke('capacity', 'write-results', {'path': str(file), 'output': result['data'], 'start_month': 4}, agent=True)
                checks['export_real_xlsx'] = code == 0 and exported.get('ok') is True and file.is_file()
                with zipfile.ZipFile(file) as archive:
                    checks['export_xlsx_crc_and_workbook'] = archive.testzip() is None and 'xl/workbook.xml' in archive.namelist()
            if name == 'privacy':
                key = 'isolated-not-a-real-amap-credential'
                payload = json.loads((ROOT / 'cli/golden/geocode/case-4/input.json').read_text())
                payload['input']['rows'] = payload['input']['rows'][:1]; payload['input']['api_key'] = key
                _, failed = invoke('geocode', 'run-batch', payload)
                text = json.dumps(failed, ensure_ascii=False)
                checks['network_denial_exercises_real_request'] = 'network_error' in text
                checks['network_errors_do_not_expose_key_or_url_query'] = key not in text and 'key=' not in text
                status = subprocess.run([str(cli), 'status', '--json'], capture_output=True, text=True, env=env, timeout=15)
                checks['account_none_and_no_key_disclosure'] = status.returncode == 0 and json.loads(status.stdout)['account']['mode'] == 'none' and key not in status.stdout
        if name == 'recovery':
            for calc, action, payload in [('missing', 'sample', {}), ('capacity', 'missing', {}), ('capacity', 'run', {}), ('capacity', 'read-excel', {'path': str(run / 'missing.xlsx')})]:
                code, body = invoke(calc, action, payload, agent=True)
                checks[f'{calc}.{action}.nonzero_readable_error'] = code != 0 and body.get('ok') is False and bool(body.get('error', {}).get('message'))
            bad = run / 'bad.xlsx'; bad.write_bytes(b'corrupt-original'); original = sha(bad)
            code, body = invoke('capacity', 'read-excel', {'path': str(bad)}, agent=True)
            checks['corrupt_input_preserved'] = code != 0 and body['ok'] is False and sha(bad) == original
            good = json.loads((ROOT / 'cli/golden/capacity/case-2/input.json').read_text())
            code, body = invoke('capacity', 'run', good, agent=True)
            checks['good_operation_recovers_after_errors'] = code == 0 and body['ok'] is True
            saved = run / 'already-saved.json'; saved.write_bytes(b'preserve-existing')
            payload = run / 'input.json'; write(payload, good)
            done = subprocess.run([str(cli), 'calc', 'capacity', 'run', '--input', str(payload), '--out', str(saved), '--json'], env=env, capture_output=True, text=True, timeout=30)
            checks['existing_output_refused_and_preserved'] = done.returncode != 0 and saved.read_bytes() == b'preserve-existing'
        if name == 'native_ui':
            done = subprocess.run([str(clone / artifact['executable']), '--ui-self-test'], env=env, capture_output=True, text=True, cwd=ROOT, timeout=90)
            (run / 'native.log').write_text(done.stdout + done.stderr)
            report = json.loads(done.stdout.strip().splitlines()[-1]); checks.update(report['checks']); cases.append({'native': report})
            checks['native_process_exit_success'] = done.returncode == 0
            assert len(report['screenshots']) >= 9
    except Exception as exc:
        error = f'{type(exc).__name__}: {exc}'
        if isinstance(exc, subprocess.CalledProcessError) and exc.stderr:
            error += ': ' + (exc.stderr.decode(errors='replace') if isinstance(exc.stderr, bytes) else exc.stderr).strip()
        checks['execution_completed'] = False
    finally:
        subprocess.run(['defaults', 'delete', domain], capture_output=True)
    raw = run / 'raw.json'; write(raw, {'schema_version': 1, 'app_id': 'hydro-mac-oss', 'check': name, 'binding': binding, 'checks': checks, 'cases': cases, 'error': error,
        'limits': ['Only synthetic samples and isolated files/domains; Amap live success requires the user-supplied service key; offline conversion and missing/blocked-key handling tested.']})
    passed = bool(checks) and all(v is True for v in checks.values())
    write(out / (name + '.detail.json'), {'summary': f'{name}: {sum(v is True for v in checks.values())}/{len(checks)} actual checks',
        'checks': checks, 'evidence': str(raw.relative_to(ROOT)), 'evidence_sha256': sha(raw), 'method': 'ui_automation' if name == 'native_ui' else 'real_product', 'error': error})
    print(f'{name}: {sum(v is True for v in checks.values())}/{len(checks)}; {error or "completed"}')
    return 0 if passed else 1

if __name__ == '__main__': raise SystemExit(main())
