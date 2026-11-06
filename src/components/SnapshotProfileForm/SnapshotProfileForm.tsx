import { Field } from '@chakra-ui/react';
import { Button } from '@/components/Button';
import { Input } from '@/components/Input';
import { NativeSelect } from '@/components/NativeSelect';
import { Textarea } from '@/components/Textarea';
import { useEffect, useRef, useState } from 'react';
import type {
  DirectorySelection,
  LibraryStatus,
  SnapshotCapabilities,
  SnapshotDestinationInput,
  SnapshotProfile,
} from '../../bindings';
import { errorMessage, libraryData, pickDirectory } from '../../ipc';
import { snapshotRequest } from '../../backup/snapshotIpc';

type Protocol = SnapshotDestinationInput['kind'];
const protocolNames: Record<Protocol, string> = {
  local: 'Local / external folder',
  sftp: 'SFTP',
  ftps: 'FTPS',
  smb: 'SMB',
  ftp: 'FTP (unencrypted transport)',
};

export function SnapshotProfileForm({
  status,
  capabilities,
  disabled,
  onSaved,
  onActivityChange,
  onClose,
}: {
  status: LibraryStatus;
  capabilities: SnapshotCapabilities | null;
  disabled: boolean;
  onSaved: (profile: SnapshotProfile) => void;
  onActivityChange: (active: boolean) => void;
  onClose: () => void;
}) {
  const base = status.roots.find((root) => root.isBase);
  const [extraRoots, setExtraRoots] = useState<Set<string>>(() => new Set());
  const [label, setLabel] = useState('');
  const [protocol, setProtocol] = useState<Protocol>('local');
  const [selection, setSelection] = useState<DirectorySelection | null>(null);
  const [host, setHost] = useState('');
  const [port, setPort] = useState('22');
  const [user, setUser] = useState('');
  const [remotePath, setRemotePath] = useState('');
  const [hostKey, setHostKey] = useState('');
  const [certificate, setCertificate] = useState('');
  const [domain, setDomain] = useState('WORKGROUP');
  const [share, setShare] = useState('');
  const [ftpAcknowledged, setFtpAcknowledged] = useState(false);
  const [repositoryRef, setRepositoryRef] = useState<string | null>(null);
  const [transportRef, setTransportRef] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const alive = useRef(true);
  const pending = useRef(false);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      onActivityChange(false);
    };
  }, [onActivityChange]);
  useEffect(() => {
    onActivityChange(busy);
  }, [busy, onActivityChange]);
  async function action(work: () => Promise<void>) {
    if (pending.current || disabled) return;
    pending.current = true;
    setBusy(true);
    setError(null);
    try {
      await work();
    } catch (cause) {
      if (alive.current) setError(errorMessage(cause));
    } finally {
      pending.current = false;
      if (alive.current) setBusy(false);
    }
  }
  function changeEndpoint(work: () => void) {
    work();
    setTransportRef(null);
  }
  function destination(): SnapshotDestinationInput | null {
    if (!capabilities?.protocols.includes(protocol)) return null;
    if (protocol === 'local')
      return selection?.identityAvailable ? { kind: 'local', selection_id: selection.id } : null;
    if (
      !host.trim() ||
      !user.trim() ||
      !/^\d{1,5}$/.test(port) ||
      Number(port) < 1 ||
      Number(port) > 65535
    )
      return null;
    const common = { host, port: Number(port), user, remote_path: remotePath };
    if (protocol === 'sftp')
      return hostKey.trim() ? { kind: 'sftp', ...common, host_key: hostKey } : null;
    if (protocol === 'ftps')
      return { kind: 'ftps', ...common, ca_certificate_pem: certificate.trim() || null };
    if (protocol === 'smb') return share.trim() ? { kind: 'smb', ...common, domain, share } : null;
    return ftpAcknowledged ? { kind: 'ftp', ...common, insecure_acknowledged: true } : null;
  }
  const target = destination();
  const canCreate = Boolean(
    base && label.trim() && repositoryRef && target && (protocol === 'local' || transportRef),
  );
  const locked = busy || disabled;
  return (
    <section className="backup-review" aria-label="New snapshot profile" aria-busy={busy}>
      <h3>New encrypted snapshot profile</h3>
      <p>
        Source: {base?.label ?? 'No base library configured'}{' '}
        <span className="backup-path">{base?.displayPath}</span>
      </p>
      <p>
        The base root is included automatically. Hidden files, companions and unknown regular files
        remain in scope; unsupported or unavailable entries must be reported.
      </p>
      {base && (
        <p>
          Base availability: {base.availability}. An unavailable root cannot be treated as empty.
        </p>
      )}
      <form
        onSubmit={(event) => {
          event.preventDefault();
          if (canCreate)
            void action(async () => {
              const target = destination();
              if (!base || !target || !repositoryRef) return;
              const value = await snapshotRequest(
                {
                  kind: 'profile_create',
                  label: label.trim(),
                  root_ids: [base.id, ...extraRoots],
                  repository_credential_ref: repositoryRef,
                  transport_credential_ref: protocol === 'local' ? null : transportRef,
                  destination: target,
                },
                'profile',
              );
              if (!alive.current) return;
              if (!value.rootIds.includes(base.id) || value.protocol !== protocol)
                throw new Error(
                  'The saved profile scope differs from this review. Refresh profiles before continuing.',
                );
              onSaved(value);
            });
        }}
      >
        <fieldset disabled={locked} className="backup-form-grid">
          <legend>Profile and source scope</legend>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Snapshot profile name</span>
              </Field.Label>
              <Input
                value={label}
                maxLength={80}
                onChange={(event) => setLabel(event.target.value)}
              />
            </label>
          </Field.Root>
          <fieldset>
            <legend>Additional roots (optional)</legend>
            {status.roots
              .filter((root) => root.id !== base?.id)
              .map((root) => (
                <label key={root.id}>
                  <input
                    type="checkbox"
                    checked={extraRoots.has(root.id)}
                    onChange={(event) => {
                      const checked = event.target.checked;
                      setExtraRoots((current) => {
                        const next = new Set(current);
                        if (checked) next.add(root.id);
                        else next.delete(root.id);
                        return next;
                      });
                    }}
                  />
                  {root.label} · {root.availability}
                </label>
              ))}
            {extraRoots.size > 0 &&
              [...extraRoots]
                .filter((id) => !status.roots.some((root) => root.id === id))
                .map((id) => (
                  <label key={id}>
                    <input
                      type="checkbox"
                      checked
                      onChange={() =>
                        setExtraRoots(
                          (current) => new Set([...current].filter((item) => item !== id)),
                        )
                      }
                    />
                    Unavailable root {id} — uncheck explicitly to remove it from this new profile
                  </label>
                ))}
            {status.roots.length <= 1 && <p>No additional roots are configured.</p>}
          </fieldset>
        </fieldset>
        <fieldset disabled={locked} className="backup-form-grid">
          <legend>Destination</legend>
          <Field.Root asChild unstyled>
            <label>
              <Field.Label asChild unstyled>
                <span>Snapshot transport</span>
              </Field.Label>
              <NativeSelect
                value={protocol}
                onChange={(event) =>
                  changeEndpoint(() => {
                    const next = event.target.value as Protocol;
                    setProtocol(next);
                    setFtpAcknowledged(false);
                    setPort(next === 'sftp' ? '22' : next === 'smb' ? '445' : '21');
                  })
                }
              >
                {(Object.keys(protocolNames) as Protocol[]).map((kind) => (
                  <option
                    key={kind}
                    value={kind}
                    disabled={!capabilities?.protocols.includes(kind)}
                  >
                    {protocolNames[kind]}
                    {!capabilities?.protocols.includes(kind) ? ' — unavailable' : ''}
                  </option>
                ))}
              </NativeSelect>
            </label>
          </Field.Root>
          {protocol === 'local' ? (
            <div>
              <p className="backup-path">
                {selection?.displayPath ?? 'Choose a separate parent folder for this repository.'}
              </p>
              <Button
                type="button"
                onClick={() =>
                  void action(async () => {
                    const value = await pickDirectory();
                    if (alive.current && value) setSelection(value);
                  })
                }
              >
                Choose snapshot destination
              </Button>
              {selection && !selection.identityAvailable && (
                <p role="alert">
                  This folder cannot supply the required native identity. Choose another
                  destination.
                </p>
              )}
            </div>
          ) : (
            <>
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Server host</span>
                  </Field.Label>
                  <Input
                    value={host}
                    maxLength={253}
                    autoComplete="off"
                    onChange={(event) => changeEndpoint(() => setHost(event.target.value))}
                  />
                </label>
              </Field.Root>
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Server port</span>
                  </Field.Label>
                  <Input
                    value={port}
                    maxLength={5}
                    inputMode="numeric"
                    onChange={(event) => changeEndpoint(() => setPort(event.target.value))}
                  />
                </label>
              </Field.Root>
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Remote username</span>
                  </Field.Label>
                  <Input
                    value={user}
                    maxLength={128}
                    autoComplete="off"
                    onChange={(event) => changeEndpoint(() => setUser(event.target.value))}
                  />
                </label>
              </Field.Root>
              <Field.Root asChild unstyled>
                <label>
                  <Field.Label asChild unstyled>
                    <span>Remote parent path</span>
                  </Field.Label>
                  <Input
                    value={remotePath}
                    maxLength={4096}
                    autoComplete="off"
                    onChange={(event) => changeEndpoint(() => setRemotePath(event.target.value))}
                  />
                </label>
              </Field.Root>
              <p>
                The backend creates a separate repository namespace. Selecting a destination never
                authorizes deleting its other contents.
              </p>
              {protocol === 'sftp' && (
                <label>
                  Verified server public host key
                  <Textarea
                    value={hostKey}
                    maxLength={16384}
                    spellCheck={false}
                    onChange={(event) => changeEndpoint(() => setHostKey(event.target.value))}
                  />
                  <small>
                    Obtain the public key through a trusted channel. A changed or unknown key blocks
                    the connection.
                  </small>
                </label>
              )}
              {protocol === 'ftps' && (
                <label>
                  Trusted CA certificate (optional PEM)
                  <Textarea
                    value={certificate}
                    maxLength={65536}
                    spellCheck={false}
                    onChange={(event) => changeEndpoint(() => setCertificate(event.target.value))}
                  />
                  <small>
                    Certificates only; never enter a private key. Empty uses the system trust store.
                    Certificate validation stays enabled.
                  </small>
                </label>
              )}
              {protocol === 'smb' && (
                <>
                  <Field.Root asChild unstyled>
                    <label>
                      <Field.Label asChild unstyled>
                        <span>SMB domain</span>
                      </Field.Label>
                      <Input
                        value={domain}
                        maxLength={128}
                        onChange={(event) => changeEndpoint(() => setDomain(event.target.value))}
                      />
                    </label>
                  </Field.Root>
                  <Field.Root asChild unstyled>
                    <label>
                      <Field.Label asChild unstyled>
                        <span>SMB share</span>
                      </Field.Label>
                      <Input
                        value={share}
                        maxLength={128}
                        onChange={(event) => changeEndpoint(() => setShare(event.target.value))}
                      />
                    </label>
                  </Field.Root>
                </>
              )}
              {protocol === 'ftp' && (
                <div role="note">
                  <p>
                    Plain FTP exposes transport credentials and connection metadata. Restic encrypts
                    backup payloads, but that does not conceal the FTP login.
                  </p>
                  <label>
                    <input
                      type="checkbox"
                      checked={ftpAcknowledged}
                      onChange={(event) =>
                        changeEndpoint(() => setFtpAcknowledged(event.target.checked))
                      }
                    />
                    I explicitly accept this unencrypted FTP transport for this profile
                  </label>
                </div>
              )}
            </>
          )}
        </fieldset>
        <fieldset disabled={locked}>
          <legend>Device credential vault</legend>
          <p>
            Passwords are entered in the native credential dialog. They are not stored in this form
            or exported with profile settings.
          </p>
          <Button
            type="button"
            disabled={!capabilities?.credentialsAvailable}
            onClick={() =>
              void action(async () => {
                const value = await libraryData(
                  { kind: 'backup_credential_enroll', purpose: 'repository', destination: null },
                  'backup_credential_ref',
                );
                if (alive.current) setRepositoryRef(value);
              })
            }
          >
            {repositoryRef
              ? 'Set a different repository password…'
              : 'Set repository recovery password…'}
          </Button>
          {repositoryRef && (
            <p>Repository recovery password saved in this device's credential vault.</p>
          )}
          {protocol !== 'local' && (
            <>
              <Button
                type="button"
                disabled={!capabilities?.credentialsAvailable || !target}
                onClick={() =>
                  void action(async () => {
                    const target = destination();
                    if (!target) return;
                    const value = await libraryData(
                      {
                        kind: 'backup_credential_enroll',
                        purpose: 'transport',
                        destination: target,
                      },
                      'backup_credential_ref',
                    );
                    if (alive.current) setTransportRef(value);
                  })
                }
              >
                {transportRef ? 'Set different remote credentials…' : 'Set remote credentials…'}
              </Button>
              {transportRef && (
                <p>
                  Remote credentials saved for these exact endpoint settings. Changing the endpoint
                  requires new native enrollment.
                </p>
              )}
            </>
          )}
          {!capabilities?.credentialsAvailable && (
            <p>{capabilities?.unavailableReason ?? 'Checking native credential availability…'}</p>
          )}
          <p>
            Keep an independent recovery copy of your repository password. Losing it can make
            encrypted backups unreadable.
          </p>
        </fieldset>
        {error && (
          <p className="error" role="alert">
            {error}
          </p>
        )}
        <div className="backup-controls">
          <Button className="primary" type="submit" disabled={locked || !canCreate}>
            Save snapshot profile
          </Button>
          <Button type="button" disabled={locked} onClick={onClose}>
            Close new profile
          </Button>
        </div>
        <p>
          Saving stores the reviewed binding only. Starting a backup requires a separate scope
          review and approval.
        </p>
      </form>
    </section>
  );
}
