"""Exercise Chapter's split code/evidence layout without launching a user App."""
import importlib.util
import json
from pathlib import Path
import plistlib
import subprocess
import sys

import pytest

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / 'scripts/accept'))
import product_checks as checks

spec = importlib.util.spec_from_file_location('hydromac_basics_acceptance', ROOT / 'scripts/accept/product_basics.py')
basics = importlib.util.module_from_spec(spec)
spec.loader.exec_module(basics)


@pytest.fixture
def layout(tmp_path, monkeypatch):
    repo, worktree = tmp_path / 'product', tmp_path / 'worktree'
    repo.mkdir(); worktree.mkdir()
    out = repo / 'perf/acceptance'
    monkeypatch.setattr(checks, 'ROOT', worktree)
    monkeypatch.setattr(basics, 'ROOT', worktree)
    monkeypatch.setattr(checks, 'registered_product', lambda: {'repo': repo})
    monkeypatch.setenv('SOP_REPO', str(worktree))
    monkeypatch.setenv('SOP_APP_ID', 'hydro-mac-oss')
    monkeypatch.setenv('SOP_OUT_DIR', str(out))
    return repo, worktree, out


def test_worktree_outputs_resolve_against_registered_product(layout):
    repo, _, out = layout
    assert checks.evidence_output('build/accept/checks') == (out, repo)
    assert not out.exists()  # Resolving the contract does not itself mutate the repo.


def test_direct_cli_defaults_remain_in_source_repository(layout, monkeypatch):
    _, worktree, _ = layout
    for name in ('SOP_OUT_DIR', 'SOP_APP_ID', 'SOP_REPO'):
        monkeypatch.delenv(name)
    assert checks.evidence_output('perf/acceptance') == (worktree / 'perf/acceptance', worktree)


@pytest.mark.parametrize('field,value', [('SOP_APP_ID', None), ('SOP_APP_ID', 'another-product'),
                                        ('SOP_REPO', None), ('SOP_REPO', '/unrelated-worktree')])
def test_external_evidence_requires_exact_product_and_worktree(layout, monkeypatch, field, value):
    if value is None:
        monkeypatch.delenv(field)
    else:
        monkeypatch.setenv(field, value)
    with pytest.raises(ValueError, match='Chapter product/worktree contract'):
        checks.evidence_output('perf/acceptance')
    assert not layout[2].exists()


@pytest.mark.parametrize('escape', ['other_product', 'parent', 'symlink'])
def test_evidence_cannot_escape_into_another_product(layout, monkeypatch, escape):
    repo, _, _ = layout
    elsewhere = repo.parent / 'another-product'; elsewhere.mkdir()
    if escape == 'symlink':
        path = repo / 'perf'; path.symlink_to(elsewhere, target_is_directory=True)
    elif escape == 'parent':
        path = repo / '../another-product'
    else:
        path = elsewhere
    monkeypatch.setenv('SOP_OUT_DIR', str(path / 'acceptance'))
    with pytest.raises(ValueError, match='registered HydroMac product'):
        checks.evidence_output('perf/acceptance')
    assert not (elsewhere / 'acceptance').exists()


@pytest.fixture
def bundle(layout, monkeypatch):
    _, worktree, _ = layout
    app = worktree / 'Product.app'
    executable = app / 'Contents/MacOS/HydroMac'
    executable.parent.mkdir(parents=True); executable.write_bytes(b'actual executable')
    info = dict(CFBundleIdentifier=checks.PRODUCT, CFBundleExecutable='HydroMac',
                CFBundleShortVersionString='1.0.1', CFBundleVersion='32')
    (app / 'Contents/Info.plist').write_bytes(plistlib.dumps(info))
    receipt = worktree / 'receipt.json'
    receipt.write_text(json.dumps({'artifact': {'bundle_id': checks.PRODUCT, 'version': '1.0.1', 'build': '32',
        'executable': 'Contents/MacOS/HydroMac', 'sha256': checks.sha(executable)},
        'source': {'sha256': 'b' * 64, 'commit': 'candidate'}}))
    monkeypatch.setenv('SOP_CODE_BINDING_SHA256', 'a' * 64)
    monkeypatch.setenv('SOP_BUILD_CURRENT', '1')
    monkeypatch.setenv('SOP_PRODUCT_BASICS_SPEC', json.dumps({'app_id': 'hydro-mac-oss', 'digest': 'spec',
        'features': {name: {'required_checks': []} for name in basics.FEATURES}}))
    monkeypatch.setattr(basics, 'owner_snapshot', lambda: {'unchanged': True})
    return app, receipt


@pytest.mark.parametrize('name', ['functionality', 'recovery', 'privacy', 'product_basics'])
def test_fixed_checks_write_readable_failure_evidence_to_product_repo(layout, bundle, monkeypatch, name):
    repo, _, out = layout
    app, receipt = bundle
    info_path = app / 'Contents/Info.plist'
    info = plistlib.loads(info_path.read_bytes()); info['CFBundleVersion'] = '31'
    info_path.write_bytes(plistlib.dumps(info))
    monkeypatch.setattr(sys, 'argv', [name + '.py', '--app', str(app), '--receipt', str(receipt)])
    calls = []
    monkeypatch.setattr(subprocess, 'run', lambda argv, **kw: calls.append(argv))
    assert (basics.main() if name == 'product_basics' else checks.main(name)) == 1
    detail = json.loads((out / (name + '.detail.json')).read_text())
    assert 'expected 32, found 31' in detail['error']
    records = detail['features'].values() if name == 'product_basics' else [detail]
    for record in records:
        raw = repo / record['evidence']
        assert raw.is_relative_to(out) and raw.is_file()
        assert checks.sha(raw) == record['evidence_sha256']
        assert 'expected 32, found 31' in json.loads(raw.read_text())['error']
    assert all(Path(argv[0]).name == 'defaults' and argv[1] == 'delete' for argv in calls)


@pytest.mark.parametrize('rejection', ['stale_production', 'invalid_signature'])
def test_basics_does_not_execute_fixtures_for_unproved_product(layout, bundle, monkeypatch, rejection):
    _, _, out = layout
    app, receipt = bundle
    monkeypatch.setattr(sys, 'argv', ['product_basics.py', '--app', str(app), '--receipt', str(receipt)])
    monkeypatch.setattr(basics.shutil, 'copytree', lambda *a, **kw: pytest.fail('Unproved product must not execute fixtures'))
    if rejection == 'stale_production':
        monkeypatch.setenv('SOP_BUILD_CURRENT', '0')
    calls = []
    def invoke(argv, **kwargs):
        calls.append(argv)
        if argv[0] == 'codesign':
            raise subprocess.CalledProcessError(1, argv, stderr=b'invalid signature')
    monkeypatch.setattr(subprocess, 'run', invoke)
    assert basics.main() == 1
    detail = json.loads((out / 'product_basics.detail.json').read_text())
    assert ('SOP_BUILD_CURRENT=0' if rejection == 'stale_production' else 'invalid signature') in detail['error']
    assert all(feature['result'] == 'failed' for feature in detail['features'].values())
    assert any(argv[0] == 'codesign' for argv in calls) == (rejection == 'invalid_signature')
