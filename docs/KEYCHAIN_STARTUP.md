# macOS Keychain and catalog startup

Stillharbour uses an exact-scope Keychain record to protect catalog activation
history, including ordinary startup. This is separate from a restic repository
password. An inaccessible record must not be treated as missing or replaced by
an empty catalog. Catalog UUIDs, credentials and retained recovery copies stay
unchanged.

Ordinary Keychain reads, writes and deletes disable implicit authentication UI
using both the SecItem query policy and the file-keychain process policy. One
native mutex protects the process-wide policy during all operations. Background
work never enables interaction. Authentication-required statuses have the typed
`CREDENTIAL_ACCESS_REQUIRED` error; other failures retain their numeric status
without exposing account names, secret values or private paths.

If catalog selection requires authorization, startup offers **Authorize** or
**Quit**. Authorize permits one foreground read of the exact activation item
bound to the held native appdata directory. macOS may request your Mac login
password. Choose **Always Allow** in that OS dialog only if you trust this build.
Stillharbour never receives that password. The OS dialog may name
`stillharbour.backup.v1`: the existing service also stores the catalog activation
record, so this label does not mean a backup is running or a repository password
is needed. Authorization does not reenroll a
backup password, rewrite an ACL itself, change the selector or create a fallback
catalog. The original selection then runs once again, with ordinary no-prompt
rules and all identity/generation/schema checks. Cancellation or a failed retry
stops startup with the catalogs retained. There is no automatic authorization
loop. A missing item after a successful OS lookup still follows the existing
history checks; a denied lookup never becomes an absent item.

If the login keychain is locked, unlock it in **Keychain Access**, then reopen
Stillharbour. Never delete a Stillharbour item, reset the login keychain or choose
“allow all applications” to get past a startup failure. Those actions cannot
establish which catalog activation is current. If an error remains, report the
message and numeric status without passwords or private filenames. Catalog
in-use, schema and stale-selector errors are separate from Keychain permission
and never offer this authorization action.

## Development rebuilds

The checked local development executable is linker/ad-hoc signed: its designated
requirement contains its code hash. Rebuilding can change that identity and
invalidate the old Keychain trust, even after Always Allow. Relaunching the same
unchanged trusted executable and authorizing a newly rebuilt executable are
different cases. No code change can promise persistent trust across these
identities while retaining the Keychain boundary.

Use a consistent valid signing identity and application identifier for bundled
builds. Tauri accepts `bundle.macOS.signingIdentity` or the
`APPLE_SIGNING_IDENTITY` environment variable for `npm run desktop:build`.
`tauri dev` does not become a signed bundle by setting that bundle option. Quit
the other instance using the same production appdata before opening a bundle;
development worktrees retain their own existing catalog locations. Switching
channels/identities may require one new explicit authorization. The current Mac
reports **zero valid code-signing identities**, so cross-rebuild signed trust is
not verified or configured by this repair. No certificate, private key, keychain
search list or system trust preference is installed or changed.

Primary references: [Apple Keychain implementations](https://developer.apple.com/documentation/technotes/tn3137-on-mac-keychains),
[process interaction policy](<https://developer.apple.com/documentation/security/seckeychainsetuserinteractionallowed(_:)>),
[code signing requirements](https://developer.apple.com/documentation/technotes/tn3127-inside-code-signing-requirements),
and [Tauri signing](https://v2.tauri.app/distribute/sign/macos/). The installed SDK
also explicitly notes that legacy keychain items may activate UI despite a
query-level no-authentication setting. This repair keeps the existing file-based
store; it introduces no Data Protection Keychain/entitlement migration.

## Validation

`node scripts/cargo.mjs test --locked -p stillharbour-core --lib backup::credentials`
checks native no-prompt policy, error classification, scoped credentials and
activation authority. The added process-policy regression fails before the fix
(allowed = 1) and passes afterward (allowed = 0). It uses only a random absent
account, never a saved user credential. `node scripts/cargo.mjs test --locked -p
stillharbour catalog_startup` verifies healthy/no-prompt startup, one authorized
retry, cancellation, failed authorization, no repeated retry and no Keychain
prompt for unrelated catalog errors.

The standard Rust/frontend/binding/release checks remain required. Native
generated catalog restart acceptance validates persistence through separate
processes using one executable. It does not prove persistent trust across ad-hoc
rebuilds, successful manual OS consent, locked-login-keychain recovery or Windows
execution. Actual results are recorded in IMPLEMENTATION_LOG.md.

## Catalog already active after the previous owner closes

A separate Unix descriptor-lifetime bug could leave the activation lock held by
an unrelated helper between fork and exec, even after its owning store closed.
ActivationLock now explicitly releases only its own held lock on final drop,
including failed initialization, matching the existing CatalogLock policy.
Catalog-backed native owners still retain it until workers and SQLite finish.
A live owner continues to reject another writer. The deterministic generated
pre-exec regression fails before this correction and passes afterward; it never
opens a personal catalog or accesses Keychain. A genuinely running second
instance still requires closing that instance. Never delete a lock file or
force-unlock an active catalog.
