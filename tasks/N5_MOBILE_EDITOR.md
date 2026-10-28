# Stillharbour - N5 - Mobile basic editor follow-on

Explicitly requested future capability; not included in the original Windows/macOS build. Prerequisites: N3 deterministic recipe contract, catalog recovery policy, and an approved mobile ADR choosing runtime and native storage/access model.

Deliver local mobile import, preview, crop, rotate, mirror, undo/revert, durable recipes and separate-copy export using the same safety rules. Handle permission revocation, partial photo-library access, OS suspension/process termination, low storage and missing original resources. Preview rendering must not rewrite the system photo library or its EXIF orientation. Export is an explicit new asset/file through tested platform APIs.

No shared live SQLite catalog over SMB/cloud drive. No implicit hosted service, account system, desktop pairing or two-way sync. Those require separate scope/security decisions. Apply minimum required OS permissions and explain them accurately.

Run LF-018 through LF-028 and the relevant catalog/import tests on actual claimed iOS/Android device/OS combinations; LF-028 covers mobile permission/suspension cases. Responsive desktop rendering is not a substitute. Gate: originals unchanged, recipes durable through suspension, output independently verified within adapter capability, limitations documented. No untested mobile support claims.
