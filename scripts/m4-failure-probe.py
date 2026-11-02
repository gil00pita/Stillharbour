#!/usr/bin/env python3
"""Actual application failure evidence on disposable loopback FTP/FTPS only."""
import argparse
import importlib.metadata
import importlib.util
import json
import os
from pathlib import Path
import secrets
import sys
import tempfile
import time

from m4_failure_server import SCENARIOS, snapshot_objects

ROOT = Path(__file__).resolve().parent.parent
SPEC = importlib.util.spec_from_file_location('m4_protocol_probe', ROOT / 'scripts/m4-protocol-probe.py')
PROBE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PROBE)


def validate_outcome(failure, scenario):
    """A lock refusal proves preservation only; never claim recovered protection."""
    if (failure.get('helpersStopped') is not True
            or failure.get('restoreNeighborPreserved') is not True
            or failure.get('localNeighborPreserved') is not True
            or failure.get('sourceFilesCompared', 0) < 5):
        raise RuntimeError('Fault preservation lacks stopped-helper or complete fixture audits.')
    if failure.get('outcome') == 'preserved_unresolved_lock':
        verification = failure.get('baselineVerification', {})
        restore = failure.get('baselineRestore', {})
        reconcile = failure.get('reconciliation', {})
        native = failure.get('nativeFailure', {})
        no_candidate = (reconcile.get('noCompletedSnapshotVisible') is True
                        and reconcile.get('snapshotPresent') is False
                        and reconcile.get('repositoryLocked') is False
                        and reconcile.get('job', {}).get('jobState') == 'needs_recovery'
                        and all(reconcile.get('job', {}).get(flag) is False for flag in
                                ('included', 'structuralChecked', 'dataReadVerified', 'restoredCompared')))
        if (failure.get('repositoryLocked') is not True
                or failure.get('recoveryCompleted') is not False
                or failure.get('baselineRestoredAfterFault') is not False
                or verification.get('state') != 'blocked_repository_locked'
                or verification.get('repositoryLocked') is not True
                or verification.get('verified') is not False
                or restore.get('state') != 'blocked_repository_locked'
                or restore.get('repositoryLocked') is not True
                or restore.get('restoredCompared') is not False
                or reconcile.get('attempted') is not True
                or (reconcile.get('repositoryLocked') is not True and not no_candidate)
                or native.get('jobState') != 'needs_recovery'
                or any(native.get(flag) is not False for flag in
                       ('included', 'structuralChecked', 'dataReadVerified', 'restoredCompared'))):
            raise RuntimeError('Unresolved lock receipt incorrectly claims recovery or lacks exact typed lock refusals.')
    elif (failure.get('outcome') not in ('preserved_verified', 'preserved_verification_rejected')
            or failure.get('repositoryLocked') is not False
            or (failure.get('outcome') == 'preserved_verification_rejected'
                and scenario != 'repository_corrupted')
            or (failure.get('outcome') == 'preserved_verified'
                and (failure.get('baselineRestoredAfterFault') is not True
                     or failure.get('recoveryCompleted') is not True))):
        raise RuntimeError('Unknown preservation outcome cannot pass as recovery evidence.')


def failure_case(protocol, scenario, application):
    nonce = secrets.token_hex(16)
    with tempfile.TemporaryDirectory(prefix=f'stillharbour-m4-failure-{nonce}-') as temporary:
        base = Path(temporary).resolve()
        (base / 'owner').write_bytes(nonce.encode())
        control = base / 'failure-control'
        control.mkdir()
        (control / 'owner').write_bytes(nonce.encode())
        server_root = base / 'server'
        server_root.mkdir()
        neighbor = server_root / 'outside-repository.bin'
        neighbor.write_bytes(b'preserved destination-only neighbor')
        password = secrets.token_urlsafe(24)
        listen_port = PROBE.port()
        environment = {name: os.environ[name] for name in ('SystemRoot', 'WINDIR', 'COMSPEC') if name in os.environ}
        environment.update({'TEMP': str(base), 'TMP': str(base), 'TMPDIR': str(base),
                            'STILLHARBOUR_PROTOCOL_SCOPE': str(base),
                            'STILLHARBOUR_PROTOCOL_OWNER': nonce,
                            'STILLHARBOUR_PROTOCOL_TOKEN': password,
                            'STILLHARBOUR_PROTOCOL_PORT': str(listen_port),
                            'STILLHARBOUR_PROTOCOL_TYPE': protocol,
                            'STILLHARBOUR_FAILURE_SCENARIO': scenario})
        destination = {'kind': protocol, 'host': '127.0.0.1', 'port': listen_port,
                       'user': 'fixture', 'remote_path': ''}
        if protocol == 'ftps':
            PROBE.fixture_certificate(base)
            destination['ca_certificate_pem'] = (base / 'server.crt').read_text()
        else:
            destination['insecure_acknowledged'] = True
        request = json.dumps({'destination': destination,
                              'repositoryPassword': secrets.token_urlsafe(32),
                              'transportPassword': password,
                              'failure': {'scenario': scenario, 'ownerNonce': nonce}})
        command = [sys.executable, ROOT / 'scripts/m4-protocol-probe.py', '--ftp-server']
        with PROBE.server(command, base, environment, listen_port):
            code, output, stderr = PROBE.run([application], base, environment,
                                              input_text=request, timeout=480, success=False)
            result = json.loads(output)
            # Retain bounded typed native evidence even if a later harness
            # assertion fails; never persist stdin credentials or raw stderr.
            native_receipt = ROOT / '.context' / f'm4-failure-application-receipt-{nonce}.json'
            native_receipt.parent.mkdir(parents=True, exist_ok=True)
            with native_receipt.open('x', encoding='utf-8') as stream:
                json.dump(result, stream, indent=2)
            if code != 0 or result.get('passed') is not True:
                diagnostic = PROBE.application_diagnostics(stderr)
                log = (base / 'server.log').read_bytes()[-1048576:].decode(errors='replace')
                phases = '; '.join(line for line in log.splitlines() if line.startswith('STILLHARBOUR_GENERATED_FAULT '))[-4096:]
                markers = ','.join(name for name in ('failure.arm','failure.disarm','failure.triggered','failure.ack') if (control / name).exists())
                raise RuntimeError(f'Actual failure probe stopped at {result.get("stage", "unknown")} '
                                   f'({result.get("errorCode", "unknown")}). {diagnostic}; controls={markers}; {phases}')
            failure = result.get('failure', {})
            validate_outcome(failure, scenario)
            if (result.get('applicationAdapter') is not True
                    or failure.get('scenario') != scenario
                    or failure.get('injectionApplied') is not True
                    or failure.get('disarmed') is not True
                    or failure.get('sourceUnchangedAfterFault') is not True
                    or failure.get('baselineHistoryRetained') is not True
                    or failure.get('baselineRestoredBeforeFault') is not True
                    or result.get('credentialCleanupVerified') is not True):
                raise RuntimeError('Actual application failure receipt lacks required preservation evidence.')
            audit_path = PROBE.owned(base, control / 'failure.audit.json')
            if audit_path.stat().st_size > 4096:
                raise RuntimeError('Fault audit exceeds its bound.')
            audit = json.loads(audit_path.read_bytes())
            if (audit.get('ownerNonce') != nonce or audit.get('scenario') != scenario
                    or audit.get('oldObjects', 0) < 1
                    or audit.get('oldObjectsUnchanged') is not True
                    or audit.get('protectedMutationAttempts') != 0):
                raise RuntimeError('Previously earned repository objects changed or a mutation was attempted.')
            frozen_path = PROBE.owned(base, control / 'failure.baseline.json')
            if frozen_path.stat().st_size > 4096:
                raise RuntimeError('Frozen generated repository baseline exceeds its bound.')
            frozen = json.loads(frozen_path.read_bytes())
            final_objects = snapshot_objects(server_root)
            if (frozen.get('ownerNonce') != nonce or frozen.get('scenario') != scenario
                    or not isinstance(frozen.get('objects'), dict)
                    or not frozen['objects'] or len(frozen['objects']) > 128
                    or any(list(final_objects.get(name, ())) != value
                           for name, value in frozen['objects'].items())
                    or (control / 'failure.protected-attempt.json').exists()):
                raise RuntimeError('Final post-reconciliation audit did not retain the exact baseline repository objects.')
            if neighbor.read_bytes() != b'preserved destination-only neighbor':
                raise RuntimeError('Destination-only neighbor changed during failure handling.')
            return {'protocol': protocol, 'scenario': scenario, 'result': 'passed',
                    'nativeOs': sys.platform, 'loopbackOnly': True,
                    'server': f'pyftpdlib {importlib.metadata.version("pyftpdlib")}',
                    'faultModel': 'owned-server-writer-ENOSPC' if scenario == 'quota_exhausted' else scenario,
                    'oldRepositoryObjectsUnchanged': True,
                    'oldRepositoryObjects': sum(name != 'outside-repository.bin'
                                                for name in frozen['objects']),
                    'destinationNeighborPreserved': True,
                    'applicationAdapter': result,
                    'productionApplicationAcceptance': False}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--protocols', nargs='+', choices=['ftp', 'ftps'], default=['ftp', 'ftps'])
    parser.add_argument('--scenarios', nargs='+', choices=SCENARIOS, default=list(SCENARIOS))
    parser.add_argument('--allow-insecure-ftp', action='store_true')
    parser.add_argument('--application-probe', type=Path, required=True)
    parser.add_argument('--evidence', type=Path, default=ROOT / '.context/m4-failure-receipt.json')
    args = parser.parse_args()
    if 'ftp' in args.protocols and not args.allow_insecure_ftp:
        raise SystemExit('Plain FTP requires --allow-insecure-ftp explicitly.')
    application = args.application_probe.resolve()
    if sys.platform == 'win32' and not application.suffix:
        application = application.with_suffix('.exe')
    name = 'backup_application_probe.exe' if sys.platform == 'win32' else 'backup_application_probe'
    if (not application.is_relative_to(ROOT / 'target') or application.name != name
            or not application.is_file() or application.is_symlink()):
        raise SystemExit('Only the built owned application probe under target/ may execute.')
    evidence = args.evidence.resolve()
    if not evidence.is_relative_to(ROOT / '.context'):
        raise SystemExit('Evidence must stay in the workspace .context directory.')
    PROBE.helpers()  # Verify both pinned native resources before starting servers.
    receipts = []
    for protocol in args.protocols:
        for scenario in args.scenarios:
            started = time.monotonic()
            try:
                receipts.append(failure_case(protocol, scenario, application))
                print(f'{protocol}/{scenario}: actual application preservation checks passed', flush=True)
            except Exception as error:
                receipts.append({'protocol': protocol, 'scenario': scenario, 'result': 'failed',
                                 'reason': str(error), 'productionApplicationAcceptance': False})
                print(f'{protocol}/{scenario}: failed; see generated receipt', flush=True)
            receipts[-1]['durationSeconds'] = round(time.monotonic() - started, 3)
            evidence.parent.mkdir(parents=True, exist_ok=True)
            evidence.write_text(json.dumps({'scope': 'generated-loopback-failures-only',
                                            'receipts': receipts}, indent=2))
    if any(item['result'] != 'passed' for item in receipts):
        raise SystemExit(1)


if __name__ == '__main__':
    main()
